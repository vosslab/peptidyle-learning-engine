# Container port mapping

This document maps the supported local Podman ports. The executable source of
truth is [containers/compose.yaml](../containers/compose.yaml). The typed
`local_stack_control` lifecycle selects the gateway host port.

All published ports bind to `127.0.0.1`. They are local-development access
points, not services exposed to the LAN.

## Default mappings

| Service            | Container port or role | Default loopback mapping | Notes                                         |
| ------------------ | ---------------------- | ------------------------ | --------------------------------------------- |
| `postgres`         | PostgreSQL `5432`      | `127.0.0.1:5432 -> 5432` | Local database access.                        |
| `minio` API        | MinIO `9000`           | `127.0.0.1:9000 -> 9000` | S3-compatible object API.                     |
| `minio` console    | MinIO `9001`           | `127.0.0.1:9001 -> 9001` | Local administration console.                 |
| `gateway`          | Caddy `8080`           | `127.0.0.1:8080 -> 8080` | The one browser and API origin.               |
| `api`              | Axum `3000`            | none                     | Gateway accesses it on private `gateway_api`. |
| `webwork-renderer` | PG renderer `3000`     | none                     | API-only private renderer endpoint.           |

The base topology uses `PLE_GATEWAY_HOST_PORT` and defaults to `8080`. The
fixed `ple-live-demo-browser` lifecycle instead selects private loopback ports
for its HTTPS gateway and prints the resulting origin. No caller-selected
project or browser/service owner is accepted. Always use the lifecycle-printed
URL for a fixed-owner browser, screenshot, or service-oracle run.

## Internal port reuse

The renderer and the API both use container port `3000`; this is not a host
port conflict. Podman gives each service its own network namespace, and the
private service name plus port identifies the destination. The API reaches the
renderer at `http://webwork-renderer:3000/`; the gateway reaches the API over
the separate internal `gateway_api` network.

`podman ps` can show `3000/tcp` without a `HOST:PORT->CONTAINER_PORT` arrow.
That is a container-local exposed-port declaration, not a host publication.
The API is reached only through the gateway; no other PLE application process
has a supported host port.

The renderer intentionally has no host-published port. Its PG/PGML request and
grading interface carries protected assessment material and remains on
`renderer_private`. Do not add a normal development mapping for it. If a
short-lived, operator-approved diagnostic ever requires host access, reserve
`127.0.0.1:8100 -> webwork-renderer:3000`; remove that override afterward.

## Reserved ranges

| Host range  | Purpose                                 | Current use                                   |
| ----------- | --------------------------------------- | --------------------------------------------- |
| `5000-5999` | Databases and supporting infrastructure | PostgreSQL uses `5432`.                       |
| `8000-8099` | Public and API gateway                  | Gateway defaults to `8080`.                   |
| `8100-8199` | Rendering diagnostics                   | `8100` is reserved; renderer remains private. |
| `9000-9099` | Object storage and administration       | MinIO API uses `9000`; console uses `9001`.   |

Avoid publishing a service merely to make container-to-container communication
work. Compose service names and their private networks already provide that
communication. Add a loopback mapping only when macOS itself needs a supported
operator-facing connection, and document the reason in the relevant operational
guide.

## Inspecting the live stack

```bash
podman compose -f containers/compose.yaml \
  --env-file containers/env.local ps
gateway_port="$(awk -F= '$1 == "PLE_GATEWAY_HOST_PORT" {print $2}' containers/env.local)"
curl -s "http://127.0.0.1:${gateway_port:-8080}/health"
```

The first command shows host-published mappings. The second uses the recorded
gateway selection rather than assuming a port. See
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for startup, health, and
recovery commands. The fixed production-auth browser, screenshot, and
service-oracle owner uses its own HTTPS origin and lifecycle commands; it does
not use this raw Compose inspection path.

## AWS baseline mapping

`deploy/opentofu/` defines this AWS mapping, but it is not live-deployment or
acceptance evidence. The topology is described in
[MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md#production-baseline-in-opentofu).

| Boundary                         | Planned port | Exposure                              | Purpose                                                                       |
| -------------------------------- | ------------ | ------------------------------------- | ----------------------------------------------------------------------------- |
| Internet to CloudFront/WAF       | `443`        | Public                                | HTTPS application entry point.                                                |
| Internet to CloudFront/WAF       | `80`         | Optional edge redirect                | HTTPS redirect only, if configured.                                           |
| CloudFront/WAF to ALB TLS origin | `443`        | Origin-facing CIDR plus secret header | Controlled TLS alias; ALB default rule denies absent/mismatched header.       |
| Private ALB to Fargate API       | `3000`       | Private target boundary               | Browser/API origin; no local gateway task.                                    |
| API to renderer                  | `443`        | Optional private integration          | Disabled unless its separately attested external renderer feature is enabled. |
| API, worker, or publisher to RDS | `5432`       | Private security groups               | TLS PostgreSQL; RDS is never public.                                          |
| API/worker/publisher to S3       | HTTPS `443`  | S3 VPC endpoint                       | No NAT route or object-storage console.                                       |
| Worker Fargate task              | none         | Private                               | No listener or target group.                                                  |

The public edge is CloudFront and WAF; the ALB, API, worker, publisher, RDS,
and S3 access remain private. The local Caddy gateway is not carried into this
topology. Private subnets have no NAT route; disabled integrations receive no
API security-group egress rule.

| Security-group owner | Inbound rule                                                                                                                   |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Private ALB          | CloudFront origin-facing managed prefix list on `443`; listener additionally requires the secret origin header.                |
| API                  | `3000` from the ALB security group only.                                                                                       |
| External renderer    | Not managed by this baseline; its feature remains disabled pending independent ingress, TLS, image, and authority attestation. |
| RDS                  | `5432` from API, worker, and publisher security groups only.                                                                   |
| Worker               | No inbound rule.                                                                                                               |
| Publisher            | No inbound rule.                                                                                                               |
| S3                   | IAM and bucket policy with an S3 VPC endpoint; no application listener.                                                        |

Repeated task-local `3000` listeners remain valid in AWS for the same reason
they do in Podman: each task has its own network namespace. Private subnets,
target groups, security groups, and service discovery define reachability. They
replace local Compose networks; they do not make the renderer or worker public.

S3 replaces local MinIO's loopback `9000` API and `9001` console. The baseline
uses IAM and four SSE-KMS bucket domains rather than a host-published object
storage administrator console. The ALB target group health-checks `/health` on
API port `3000` and allows a 45-second drain, longer than the API's 30-second
graceful request drain. Browser WebSocket behavior still needs separate
acceptance evidence if introduced.

### Primary AWS references

- [Application Load Balancer listeners](https://docs.aws.amazon.com/elasticloadbalancing/latest/application/load-balancer-listeners.html)
- [Application Load Balancer redirect actions](https://docs.aws.amazon.com/elasticloadbalancing/latest/application/rule-action-types.html)
- [Application Load Balancer security groups](https://docs.aws.amazon.com/elasticloadbalancing/latest/application/load-balancer-update-security-groups.html)
- [Application Load Balancer target health checks](https://docs.aws.amazon.com/elasticloadbalancing/latest/application/target-group-health-checks.html)
- [Amazon ECS service discovery](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/service-discovery.html)
- [Amazon S3 VPC endpoints](https://docs.aws.amazon.com/vpc/latest/privatelink/vpc-endpoints-s3.html)
