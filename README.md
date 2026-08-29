# vout

vout is an open-source, local implementation of the AWS Systems Manager (SSM)
Parameter Store API. It is useful for local development and tests that need
parameter storage without connecting to AWS. This is a 50% AI project.

## Run

Build and start the service with a YAML configuration file:

```sh
cargo run -p vout -- --config ./vout.yaml
```

Example `vout.yaml`:

```yaml
db_dsn: "sqlite:vout.sqlite?mode=rwc"
amz_error_on_unsupported: true

first_user:
  access_key: "local-access-key"
  secret_key: "local-secret-key"
  lifetime: 60

server:
  listen_address: "127.0.0.1:4566"
  concurrent: 50
  timeout: 30s
  max_body_bytes: 1048576
```

The service exposes health checks at `/health` and `/ready`. Parameters are
addressed by their exact name. A path such as `brick` is used with
`get-parameters-by-path` to find names below it, for example `brick/layer`.

To clear the local database encryption key from the OS keyring and exit:

```sh
cargo run -p vout -- --clear-keyring
```

## AWS CLI

Configure credentials for the local server, then create a parameter before
reading it:

```sh
aws configure set aws_access_key_id local-access-key --profile vout
aws configure set aws_secret_access_key local-secret-key --profile vout

AWS_PROFILE=vout AWS_DEFAULT_REGION=local AWS_ENDPOINT_URL=http://127.0.0.1:4566 \
  aws ssm put-parameter --name brick/layer --type SecureString --value secret --overwrite

AWS_PROFILE=vout AWS_DEFAULT_REGION=local AWS_ENDPOINT_URL=http://127.0.0.1:4566 \
  aws ssm get-parameter --name brick/layer --with-decryption

AWS_PROFILE=vout AWS_DEFAULT_REGION=local AWS_ENDPOINT_URL=http://127.0.0.1:4566 \
  aws ssm get-parameters-by-path --path brick --recursive
```

`get-parameters --names` performs exact lookups, so `brick` and `brick/` do
not match the existing parameter `brick/layer`.

## Environment variables

Configuration can also be supplied with environment variables prefixed with
`VOUT_`. Nested configuration fields use `-` as the separator. Because `-` is
not valid in normal shell variable assignments, use `env` for nested settings:
vout reads the process environment; it does not load a `.env` file by itself.

```sh
env \
  VOUT_DB_DSN="sqlite:vout.sqlite?mode=rwc" \
  VOUT_AMZ_ERROR_ON_UNSUPPORTED=true \
  'VOUT_FIRST_USER-ACCESS_KEY=local-access-key' \
  'VOUT_FIRST_USER-SECRET_KEY=local-secret-key' \
  'VOUT_SERVER-LISTEN_ADDRESS=127.0.0.1:4566' \
  'VOUT_SERVER-CONCURRENT=50' \
  'VOUT_SERVER-TIMEOUT=30s' \
  'VOUT_SERVER-MAX_BODY_BYTES=1048576' \
  cargo run -p vout
```

The available variables are:

| Variable | Default | Description |
| --- | --- | --- |
| `VOUT_DB_DSN` | `sqlite::memory:` | SeaORM database connection string |
| `VOUT_AMZ_ERROR_ON_UNSUPPORTED` | `true` | Return errors for unsupported AWS fields |
| `VOUT_FIRST_USER-ACCESS_KEY` | unset | Initial AWS access key |
| `VOUT_FIRST_USER-SECRET_KEY` | unset | Initial AWS secret key |
| `VOUT_FIRST_USER-LIFETIME` | unset | Initial credential lifetime in minutes |
| `VOUT_SERVER-LISTEN_ADDRESS` | `127.0.0.1:4566` | Bind address |
| `VOUT_SERVER-CONCURRENT` | `50` | Maximum concurrent requests |
| `VOUT_SERVER-TIMEOUT` | `30s` | Request timeout |
| `VOUT_SERVER-MAX_BODY_BYTES` | `1048576` | Maximum request body size |
| `VOUT_SERVER-TLS-CERT_FILENAME` | unset | TLS certificate path |
| `VOUT_SERVER-TLS-KEY_FILENAME` | unset | TLS private key path |

For TLS, set both certificate variables. Environment variables are loaded
before the configuration file, so values in the YAML file take precedence when
the same setting is present in both.

## boto3

```python
import boto3

ssm = boto3.client(
    "ssm",
    endpoint_url="http://127.0.0.1:4566",
    region_name="local",
    aws_access_key_id="local-access-key",
    aws_secret_access_key="local-secret-key",
)

ssm.put_parameter(
    Name="brick/layer",
    Type="SecureString",
    Value="secret",
    Overwrite=True,
)
print(ssm.get_parameter(Name="brick/layer", WithDecryption=True))
```

## AWS SDK for JavaScript

```ts
import { PutParameterCommand, SSMClient } from "@aws-sdk/client-ssm";

const ssm = new SSMClient({
  endpoint: "http://127.0.0.1:4566",
  region: "local",
  credentials: {
    accessKeyId: "local-access-key",
    secretAccessKey: "local-secret-key",
  },
});

await ssm.send(new PutParameterCommand({
  Name: "brick/layer",
  Type: "SecureString",
  Value: "secret",
  Overwrite: true,
}));
```

Supported SSM operations include putting, reading, listing, describing,
versioning, labeling, and deleting parameters. The integration tests in
`dev/amz-api-tests` exercise the AMZ-compatible API.

```sh
cd dev/amz-api-tests
bun install
bun test
```
