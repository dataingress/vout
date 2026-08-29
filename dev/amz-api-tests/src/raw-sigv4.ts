import { createHash, createHmac } from "node:crypto";
import { credentials, region } from "./harness";

const algorithm = "AWS4-HMAC-SHA256";
const service = "ssm";

function sha256Hex(value: string): string {
  return createHash("sha256").update(value).digest("hex");
}

function hmac(key: Buffer | string, value: string): Buffer {
  return createHmac("sha256", key).update(value).digest();
}

function signingKey(secretAccessKey: string, date: string): Buffer {
  const kDate = hmac(`AWS4${secretAccessKey}`, date);
  const kRegion = hmac(kDate, region);
  const kService = hmac(kRegion, service);
  return hmac(kService, "aws4_request");
}

function amzDate(now = new Date()): string {
  return now.toISOString().replace(/[:-]|\.\d{3}/g, "");
}

export async function signedAmzFetch(
  endpoint: string,
  target: string,
  rawBody: string,
): Promise<Response> {
  const url = new URL("/", endpoint);
  const dateTime = amzDate();
  const date = dateTime.slice(0, 8);
  const payloadHash = sha256Hex(rawBody);
  const headers = {
    "content-type": "application/x-amz-json-1.1",
    host: url.host,
    "x-amz-content-sha256": payloadHash,
    "x-amz-date": dateTime,
    "x-amz-target": target,
  };
  const signedHeaders = Object.keys(headers).sort();
  const canonicalHeaders = signedHeaders
    .map((name) => `${name}:${headers[name as keyof typeof headers]}`)
    .join("\n");
  const canonicalRequest = [
    "POST",
    "/",
    "",
    `${canonicalHeaders}\n`,
    signedHeaders.join(";"),
    payloadHash,
  ].join("\n");
  const scope = `${date}/${region}/${service}/aws4_request`;
  const stringToSign = [
    algorithm,
    dateTime,
    scope,
    sha256Hex(canonicalRequest),
  ].join("\n");
  const signature = createHmac("sha256", signingKey(credentials.secretAccessKey, date))
    .update(stringToSign)
    .digest("hex");
  const authorization = `${algorithm} Credential=${credentials.accessKeyId}/${scope}, SignedHeaders=${signedHeaders.join(";")}, Signature=${signature}`;

  return await fetch(url, {
    method: "POST",
    headers: {
      ...headers,
      authorization,
    },
    body: rawBody,
  });
}

export async function rawAmzJson(
  endpoint: string,
  target: string,
  body: unknown,
): Promise<{ status: number; json: any }> {
  const response = await signedAmzFetch(endpoint, target, JSON.stringify(body));
  return {
    status: response.status,
    json: await response.json(),
  };
}
