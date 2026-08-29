import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import {
  DeleteParameterCommand,
  DeleteParametersCommand,
  DescribeParametersCommand,
  GetParameterCommand,
  GetParameterHistoryCommand,
  GetParametersByPathCommand,
  GetParametersCommand,
  LabelParameterVersionCommand,
  PutParameterCommand,
  SSMClient,
  UnlabelParameterVersionCommand,
} from "@aws-sdk/client-ssm";
import { expectAwsError, startVout, type TestServer } from "./harness";
import { rawAmzJson, signedAmzFetch } from "./raw-sigv4";

const target = {
  deleteParameter: "AmazonSSM.DeleteParameter",
  describeParameters: "AmazonSSM.DescribeParameters",
  putParameter: "AmazonSSM.PutParameter",
};

describe("vout service endpoints", () => {
  let server: TestServer;

  beforeAll(async () => {
    server = await startVout();
  }, 60_000);

  afterAll(async () => {
    if (server) {
      await server.stop();
    }
  });

  test("reports health and readiness", async () => {
    const health = await fetch(`${server.endpoint}/health`);
    expect(health.status).toBe(200);
    expect(await health.text()).toBe("OK");

    const ready = await fetch(`${server.endpoint}/ready`);
    expect(ready.status).toBe(200);
    expect(await ready.json()).toMatchObject({ ready: true });
  });

  test("creates, overwrites, reads latest, reads by version, and rejects duplicate create", async () => {
    const first = await server.ssm.send(new PutParameterCommand({
      Name: "/app/plain",
      Value: "v1",
      Type: "String",
      Description: "plain parameter",
      DataType: "text",
      AllowedPattern: "^v[0-9]+$",
      Tags: [{ Key: "suite", Value: "amz-api" }],
    }));
    expect(first.Version).toBe(1);

    await expectAwsError(
      server.ssm.send(new PutParameterCommand({
        Name: "/app/plain",
        Value: "v1-again",
        Type: "String",
      })),
      "ParameterAlreadyExists",
    );

    const second = await server.ssm.send(new PutParameterCommand({
      Name: "/app/plain",
      Value: "v2",
      Type: "String",
      Overwrite: true,
      Tags: [{ Key: "suite", Value: "amz-api-updated" }],
    }));
    expect(second.Version).toBe(2);

    const latest = await server.ssm.send(new GetParameterCommand({ Name: "/app/plain" }));
    expect(latest.Parameter).toMatchObject({
      Name: "/app/plain",
      Type: "String",
      Value: "v2",
      Version: 2,
      DataType: "text",
    });

    const byVersion = await server.ssm.send(new GetParameterCommand({ Name: "/app/plain:1" }));
    expect(byVersion.Parameter).toMatchObject({
      Name: "/app/plain",
      Selector: ":1",
      Value: "v1",
      Version: 1,
    });
  });

  test("handles SecureString encryption and decryption", async () => {
    await server.ssm.send(new PutParameterCommand({
      Name: "/app/secret",
      Value: "secret-value",
      Type: "SecureString",
    }));

    const encrypted = await server.ssm.send(new GetParameterCommand({
      Name: "/app/secret",
    }));
    expect(encrypted.Parameter?.Value).not.toBe("secret-value");

    const decrypted = await server.ssm.send(new GetParameterCommand({
      Name: "/app/secret",
      WithDecryption: true,
    }));
    expect(decrypted.Parameter?.Value).toBe("secret-value");
  });

  test("labels, reads by label, unlabels, and reports invalid labels", async () => {
    await server.ssm.send(new PutParameterCommand({
      Name: "/app/labeled",
      Value: "one",
      Type: "String",
    }));
    await server.ssm.send(new PutParameterCommand({
      Name: "/app/labeled",
      Value: "two",
      Type: "String",
      Overwrite: true,
    }));

    const label = await server.ssm.send(new LabelParameterVersionCommand({
      Name: "/app/labeled",
      Labels: ["current", "1bad"],
      ParameterVersion: 1,
    }));
    expect(label.ParameterVersion).toBe(1);
    expect(label.InvalidLabels).toEqual(["1bad"]);

    const byLabel = await server.ssm.send(new GetParameterCommand({
      Name: "/app/labeled:current",
    }));
    expect(byLabel.Parameter).toMatchObject({
      Selector: ":current",
      Value: "one",
      Version: 1,
    });

    const unlabel = await server.ssm.send(new UnlabelParameterVersionCommand({
      Name: "/app/labeled",
      Labels: ["current", "missing"],
      ParameterVersion: 1,
    }));
    expect(unlabel.RemovedLabels).toEqual(["current"]);
    expect(unlabel.InvalidLabels).toEqual(["missing"]);
  });

  test("returns parameter history with pagination and labels", async () => {
    await server.ssm.send(new PutParameterCommand({
      Name: "/app/history",
      Value: "first",
      Type: "StringList",
    }));
    await server.ssm.send(new PutParameterCommand({
      Name: "/app/history",
      Value: "second",
      Type: "StringList",
      Overwrite: true,
    }));
    await server.ssm.send(new LabelParameterVersionCommand({
      Name: "/app/history",
      Labels: ["stable"],
      ParameterVersion: 2,
    }));

    const page1 = await server.ssm.send(new GetParameterHistoryCommand({
      Name: "/app/history",
      MaxResults: 1,
    }));
    expect(page1.Parameters).toHaveLength(1);
    expect(page1.Parameters?.[0]).toMatchObject({ Value: "first", Version: 1 });
    expect(page1.NextToken).toBe("1");

    const page2 = await server.ssm.send(new GetParameterHistoryCommand({
      Name: "/app/history",
      NextToken: page1.NextToken,
    }));
    expect(page2.Parameters?.[0]).toMatchObject({
      Value: "second",
      Version: 2,
      Labels: ["stable"],
    });
  });

  test("gets batches with sorted found and invalid parameters", async () => {
    await server.ssm.send(new PutParameterCommand({
      Name: "/batch/a",
      Value: "A",
      Type: "String",
    }));
    await server.ssm.send(new PutParameterCommand({
      Name: "/batch/b",
      Value: "B",
      Type: "String",
    }));

    const result = await server.ssm.send(new GetParametersCommand({
      Names: ["/batch/b", "/batch/missing", "/batch/a", "bad name"],
    }));
    expect(result.Parameters?.map((param) => param.Name)).toEqual(["/batch/a", "/batch/b"]);
    expect(result.InvalidParameters).toEqual(["/batch/missing", "bad name"]);
  });

  test("lists by path recursively, one level, with label/type filters and pagination", async () => {
    await server.ssm.send(new PutParameterCommand({
      Name: "/tree/root",
      Value: "root",
      Type: "String",
    }));
    await server.ssm.send(new PutParameterCommand({
      Name: "/tree/child/secure",
      Value: "child",
      Type: "SecureString",
    }));
    await server.ssm.send(new LabelParameterVersionCommand({
      Name: "/tree/root",
      Labels: ["pathlabel"],
    }));

    const oneLevel = await server.ssm.send(new GetParametersByPathCommand({
      Path: "/tree",
      Recursive: false,
    }));
    expect(oneLevel.Parameters?.map((param) => param.Name)).toEqual(["/tree/root"]);

    const recursivePage = await server.ssm.send(new GetParametersByPathCommand({
      Path: "/tree",
      Recursive: true,
      MaxResults: 1,
    }));
    expect(recursivePage.Parameters).toHaveLength(1);
    expect(recursivePage.NextToken).toBe("1");

    const labelFiltered = await server.ssm.send(new GetParametersByPathCommand({
      Path: "/tree",
      Recursive: true,
      ParameterFilters: [{ Key: "Label", Option: "Equals", Values: ["pathlabel"] }],
    }));
    expect(labelFiltered.Parameters?.map((param) => param.Name)).toEqual(["/tree/root"]);

    const typeFiltered = await server.ssm.send(new GetParametersByPathCommand({
      Path: "/tree",
      Recursive: true,
      ParameterFilters: [{ Key: "Type", Option: "BeginsWith", Values: ["Secure"] }],
      WithDecryption: true,
    }));
    expect(typeFiltered.Parameters?.map((param) => param.Name)).toEqual(["/tree/child/secure"]);
    expect(typeFiltered.Parameters?.[0].Value).toBe("child");
  });

  test("describes metadata with supported filters and pagination", async () => {
    await server.ssm.send(new PutParameterCommand({
      Name: "/describe/alpha",
      Value: "alpha",
      Type: "String",
      Description: "alpha description",
      DataType: "text",
      Tags: [{ Key: "role", Value: "alpha" }],
    }));
    await server.ssm.send(new PutParameterCommand({
      Name: "/describe/beta",
      Value: "beta",
      Type: "SecureString",
      Tags: [{ Key: "role", Value: "beta" }],
    }));

    const byName = await server.ssm.send(new DescribeParametersCommand({
      ParameterFilters: [{ Key: "Name", Option: "BeginsWith", Values: ["/describe/a"] }],
    }));
    expect(byName.Parameters?.map((param) => param.Name)).toEqual(["/describe/alpha"]);
    expect(byName.Parameters?.[0]).toMatchObject({
      Description: "alpha description",
      Tier: "Standard",
      DataType: "text",
    });

    const byPath = await server.ssm.send(new DescribeParametersCommand({
      ParameterFilters: [{ Key: "Path", Option: "OneLevel", Values: ["/describe"] }],
      MaxResults: 1,
    }));
    expect(byPath.Parameters).toHaveLength(1);
    expect(byPath.NextToken).toBe("1");

    const byTag = await server.ssm.send(new DescribeParametersCommand({
      ParameterFilters: [{ Key: "tag:role", Option: "Equals", Values: ["beta"] }],
    }));
    expect(byTag.Parameters?.map((param) => param.Name)).toEqual(["/describe/beta"]);

    const byTier = await server.ssm.send(new DescribeParametersCommand({
      ParameterFilters: [{ Key: "Tier", Option: "Equals", Values: ["Standard"] }],
    }));
    expect(byTier.Parameters?.some((param) => param.Name === "/describe/alpha")).toBe(true);
  });

  test("deletes one parameter and deletes many with invalid entries", async () => {
    await server.ssm.send(new PutParameterCommand({
      Name: "/delete/one",
      Value: "one",
      Type: "String",
    }));
    await server.ssm.send(new PutParameterCommand({
      Name: "/delete/many-a",
      Value: "a",
      Type: "String",
    }));
    await server.ssm.send(new PutParameterCommand({
      Name: "/delete/many-b",
      Value: "b",
      Type: "String",
    }));

    const one = await server.ssm.send(new DeleteParameterCommand({ Name: "/delete/one" }));
    expect(one.$metadata.httpStatusCode).toBe(200);
    await expectAwsError(
      server.ssm.send(new GetParameterCommand({ Name: "/delete/one" })),
      "ParameterNotFound",
    );

    const many = await server.ssm.send(new DeleteParametersCommand({
      Names: ["/delete/many-a", "/delete/missing", "/delete/many-b"],
    }));
    expect(many.DeletedParameters).toEqual(["/delete/many-a", "/delete/many-b"]);
    expect(many.InvalidParameters).toEqual(["/delete/missing"]);
  });

  test("returns expected AMZ protocol errors", async () => {
    await expectAwsError(
      server.ssm.send(new GetParameterCommand({ Name: "/missing" })),
      "ParameterNotFound",
    );
    await expectAwsError(
      server.ssm.send(new PutParameterCommand({
        Name: "/bad/pattern",
        Value: "abc",
        Type: "String",
        AllowedPattern: "^[0-9]+$",
      })),
      "InvalidAllowedPatternException",
    );
    await expectAwsError(
      server.ssm.send(new DescribeParametersCommand({
        ParameterFilters: [{ Key: "Unknown", Option: "Equals", Values: ["x"] }],
      })),
      "InvalidFilterKey",
    );
    await expectAwsError(
      server.ssm.send(new DescribeParametersCommand({
        NextToken: "not-a-number",
      })),
      "InvalidNextTokenException",
    );

    const unsupported = await rawAmzJson(server.endpoint, target.putParameter, {
      Name: "/unsupported",
      Value: "value",
      Type: "String",
      Tier: "Advanced",
    });
    expect(unsupported.status).toBe(400);
    expect(unsupported.json.__type).toBe("InvalidParameterException");
    expect(unsupported.json.Message).toContain("Unsupported request field 'Tier'");

    const malformed = await signedAmzFetch(
      server.endpoint,
      target.describeParameters,
      "{",
    );
    expect(malformed.status).toBe(400);
    expect((await malformed.json()).__type).toBe("InvalidParameterException");

    const unauthenticated = await fetch(`${server.endpoint}/`, {
      method: "POST",
      headers: { "x-amz-target": target.deleteParameter },
      body: "{}",
    });
    expect(unauthenticated.status).toBe(400);
    expect((await unauthenticated.json()).Message).toBe("Missing Authorization header.");
  });

  test("reports unknown routes", async () => {
    const response = await fetch(`${server.endpoint}/unknown`);
    expect(response.status).toBe(400);
    expect((await response.json()).__type).toBe("UnknownRoute");
  });
});

test("SDK client can be configured for the local AMZ-compatible endpoint", async () => {
  const client = new SSMClient({
    endpoint: "http://127.0.0.1:4566",
    region: "local",
    credentials: {
      accessKeyId: "example",
      secretAccessKey: "example",
    },
  });

  await expect(client.config.region()).resolves.toBe("local");
});
