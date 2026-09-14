# Gmail API Email Backend Specification

## Status

Proposed specification for a supported PLE Email Delivery Backend.

This document does not claim that Gmail delivery, email-code authentication, OAuth setup commands,
or connected email acceptance exist today. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) remains the
product authority, [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns PLE vocabulary, and
[ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md) owns Account, Course Roster Import, and Course
Invitation behavior.

The Gmail API backend is intended to support small PLE installations without requiring a paid
email-delivery service. It may begin as the backend for a Pilot deployment, but its design is
suitable for continued production use while message volume, provider policy, reliability, and
deliverability remain acceptable.

Gmail is an email transport provider only. Gmail does not own PLE Accounts, authentication
decisions, Product Roles, Authenticated Sessions, enrollment, or authorization.

## Goals

- Provide passwordless PLE authentication email through a dedicated Gmail account.
- Keep PLE Accounts independent of Google identity.
- Support Student authentication at explicitly permitted university email domains.
- Keep Gmail-specific behavior behind a replaceable Email Delivery Backend.
- Make installation and recovery practical for a deployment operator without adding a rarely used
  administrative webpage.
- Keep deployment credentials outside the repository and ordinary application data.
- Keep the initial deployment inexpensive and operationally simple.
- Allow migration to another provider without changing PLE Accounts, Authenticated Sessions, or
  authentication workflows.

## Non-goals

The Gmail backend does not provide Sign in with Google, Google Workspace single sign-on, Gmail
mailbox access, a PLE mail server, a general-purpose email administration interface, or a general
platform secret-management system.

PLE does not build infrastructure for hypothetical future providers. A provider-neutral email
delivery boundary is sufficient to preserve that option.

The backend does not create Accounts from arbitrary email addresses. Student Account creation and
Student Authentication Email assignment remain part of Course Roster Import. Instructor Account
creation remains a vetted Sysadmin operation.

## Architectural boundary

PLE authentication owns the authentication workflow. The Email Delivery Backend only delivers a
bounded message to an address selected by PLE.

```text
PLE authentication
    |
    | send(message)
    v
Email Delivery Backend
    |
    +-- Gmail API backend
    |
    `-- possible future provider backend
```

The Gmail backend receives a completed delivery request containing the recipient, message kind,
fixed subject, and bounded message body. It does not decide whether the recipient may authenticate,
create or validate challenges, create sessions, determine Product Roles, inspect Course
Membership, or authorize Student Work.

No Gmail-specific concept appears in the Account, authentication-challenge, or session model.
Provider replacement changes installation configuration and credentials, not product identity.

## Provider-neutral delivery contract

The server owns one narrow operation equivalent to:

```rust
pub trait EmailDeliveryBackend {
    async fn send(
        &self,
        message: EmailDeliveryMessage,
    ) -> Result<EmailDeliveryReceipt, EmailDeliveryFailure>;
}
```

`EmailDeliveryMessage` is a closed application type. It contains one validated recipient, one
supported message kind, and server-rendered content. It does not accept an arbitrary provider URL,
OAuth scope, mail header, HTML body, attachment, or credential.

`EmailDeliveryReceipt` means that the provider accepted the submission. It does not prove inbox
delivery or successful authentication.

Provider-specific errors remain inside the adapter. The adapter returns bounded operational
categories such as configuration, credential, quota, recipient, transient, indeterminate, and
protocol failure. Provider response bodies and OAuth error descriptions never enter browser
responses or ordinary logs.

## Student authentication flow

A typical Student sign-in is:

1. The Student enters the required university email address.
2. PLE normalizes and validates the address according to the configured institutional-domain
   policy.
3. PLE resolves an eligible existing Account without revealing whether the Account exists.
4. PLE creates a cryptographically random, short-lived, single-use, browser-bound authentication
   challenge.
5. PLE stores only the challenge hash and the bounded state required to validate purpose, browser
   binding, expiration, attempts, and consumption.
6. PLE asks the configured Email Delivery Backend to deliver the authentication message.
7. The Gmail backend sends the message from the dedicated PLE Gmail account to the Student's
   university mailbox.
8. The Student follows the link or enters the code in the requesting browser.
9. PLE validates and consumes the challenge.
10. PLE creates its own Authenticated Session.
11. PLE derives authorization and Product Role from the PLE Account and exact product
    relationships.

Receiving a message at a university address proves control of that mailbox for the limited purpose
defined by PLE. It does not make Google an identity provider.

Students do not need Gmail accounts and do not authorize PLE to access Google. A Student may
authenticate as `student@mail.roosevelt.edu` while the message is sent from a dedicated
`@gmail.com` account.

Course Roster Import permits exact configured domains such as `mail.roosevelt.edu`. A suffix such
as `.edu` is not an authorization rule. The public authentication-start response must not reveal
whether an address belongs to an Account, is inactive, is outside the permitted domain, or is
currently rate-limited (ASVS 6.3.1, 6.6.2-6.6.3).

## Authentication-message content

Authentication messages contain no Student name, Course name, roster ID, Account identifier,
Product Role, grade, Assignment, or Student Work. A message contains only the installation name,
authentication link or code, expiration information, and instructions to ignore an unrequested
message.

The initial backend sends plain text without attachments, tracking pixels, remote images,
marketing content, `Cc`, or `Bcc`. Message construction uses typed header setters so recipient data
cannot inject a mail header (ASVS 1.1.1-1.1.2, 1.2.1-1.2.5).

## Sender account

Each installation using the Gmail backend uses a dedicated Gmail account rather than a deployment
operator's personal mailbox. For example:

```text
peptidyle.auth@gmail.com
```

The exact address is installation configuration, not application logic.

The dedicated account is used only for PLE operational email. It contains no ordinary personal,
course, grade, or Student correspondence. The deployment operator enables two-factor
authentication and retains account-recovery information outside PLE.

Google may retain sent-message content and recipient metadata under the Gmail account's policies.
PLE limits that exposure by sending bounded messages without educational records. If provider
retention or institutional policy becomes unacceptable, the installation selects another backend.

## Google authorization

The dedicated sender account authorizes PLE. Students and Instructors do not participate in this
Google authorization flow.

PLE requests only these permissions:

```text
openid
email
https://www.googleapis.com/auth/gmail.send
```

`gmail.send` is the only Gmail permission. PLE does not request permission to read, modify, or
delete mailbox content. `openid email` lets the installer verify that the exact configured sender
authorized PLE; Google identity in this installer ceremony is not Student authentication. See
Google's [Gmail scope list](https://developers.google.com/workspace/gmail/api/auth/scopes).

The resulting OAuth refresh token is an installation credential. PLE uses it to obtain short-lived
access tokens as required by Google. Access and refresh tokens remain in the trusted backend and
never enter browser state, application data, or product authorization (ASVS 9.1.1-9.1.3,
10.5.1-10.5.4).

## Installation model

Google-side account and project creation remains a manual deployment-operator responsibility. PLE
automates the credential exchange and validation that benefits from application knowledge.

The expected installation sequence is:

1. Create or select the dedicated Gmail sender account.
2. Enable two-factor authentication and preserve its recovery information.
3. Create a Google Cloud project and enable the Gmail API.
4. Configure the OAuth consent screen and production publishing state.
5. Create a Desktop application OAuth client.
6. Download the Google OAuth client configuration.
7. Run the PLE Gmail authorization command.
8. Authenticate to Google as the dedicated sender account.
9. Grant the required send permission.
10. Allow the PLE command to receive and validate the authorization result.
11. Store the resulting installation credential in the protected PLE secret location.
12. Send a test message to real university destinations.
13. Configure the installation to use the Gmail API backend.

Google Cloud project administration remains in Google's interfaces. PLE does not reproduce project
or OAuth-client administration in the Sysadmin web interface.

The operator must follow Google's current publishing and verification requirements. In particular,
an External OAuth application left in Testing status normally receives refresh tokens that expire
after seven days. See Google's
[OAuth application states](https://developers.google.com/identity/protocols/oauth2/production-readiness/overview)
and [verification exceptions](https://support.google.com/cloud/answer/13464323).

## Operator CLI

Gmail configuration is rare installation and recovery work. It uses PLE's existing Rust operator
tooling rather than a permanent web administration surface.

The supported command family is:

```text
cargo tools email-delivery gmail authorize --client-credentials <path>
cargo tools email-delivery gmail reauthorize --client-credentials <path>
cargo tools email-delivery status
cargo tools email-delivery verify
cargo tools email-delivery send-test <address>
```

The human running these commands is the deployment operator. That person may also have a PLE
Sysadmin Account, but the Sysadmin Product Role alone grants no host or Gmail credential authority.

### Authorize and reauthorize

`authorize` performs the interactive OAuth exchange for the dedicated sender account. It:

- reads the downloaded Google OAuth client configuration from an explicit file argument;
- starts Google's supported Desktop application authorization-code flow;
- listens temporarily on a random `127.0.0.1` loopback port;
- validates OAuth state, PKCE, OIDC nonce, issuer, audience, expiry, subject, verified email, and
  granted scopes;
- verifies the authorized email exactly matches the configured sender;
- writes the resulting installation credential to the protected credential file;
- avoids printing tokens or client secrets; and
- reports successful authorization and the credential destination without exposing secret values.

The command does not use an embedded browser, public PLE callback route, fixed listening port, or
retired manual copy-and-paste flow. Google's
[Desktop OAuth guidance](https://developers.google.com/identity/protocols/oauth2/native-app) and
[OAuth security policy](https://developers.google.com/identity/protocols/oauth2/policies) own the
external ceremony.

Initial authorization refuses to overwrite an existing credential. `reauthorize` writes and
validates a private candidate, then atomically replaces the active file. Failed reauthorization
leaves the previous credential unchanged.

Reauthorization is the normal recovery path for a revoked, lost, expired, or deliberately replaced
credential. It does not change PLE Accounts or Authenticated Sessions.

### Status and verify

`status` performs a local, redacted inspection without contacting Google. It reports information
equivalent to:

```text
Backend: Gmail API
Configured: yes
Credential file: readable
Sender: peptidyle.auth@gmail.com
Local credential: valid shape
```

`verify` contacts Google, refreshes an access token, and confirms the authorized sender and granted
scope without sending mail. It distinguishes missing configuration, missing or unreadable files,
invalid credential format, revoked authorization, sender mismatch, scope mismatch, and unavailable
Google service.

Neither command prints access tokens, refresh tokens, OAuth client secrets, provider response
bodies, or authentication challenges.

### Send test

`send-test` sends one clearly marked message through the same Email Delivery Backend used by PLE.
The destination is an explicit validated command argument; the command does not read a roster or
send a bulk test.

An installation intended to authenticate university users tests delivery to representative real
addresses in the permitted institutional domain. Successful API submission alone does not establish
that institutional spam filtering will deliver the message to the inbox.

## Credential storage

PLE does not need a general platform secret-management system solely to support Gmail.

The Gmail OAuth credential is an installation-owned host secret stored outside the repository and
ordinary application data. A conventional system deployment may use:

```text
/etc/ple/
    ple.toml
    secrets/
        gmail-oauth.json
```

The exact location is deployment-specific. The existing local deployment may place the credential
under its controller-owned `.secrets` directory.

The secret directory is owned by the deployment account, is not a symbolic link, and uses mode
`0700` where supported. The credential is a regular non-symbolic-link file owned by the deployment
account with exact mode `0600`. PLE bounds the file size, strictly validates its serialized shape,
and refuses an unsafe, unreadable, or malformed credential (ASVS 2.1.1-2.1.3, 14.2.1-14.2.7).

Normal configuration stores only non-secret information and the credential location. Conceptually:

```toml
[email]
backend = "gmail_api"
sender = "peptidyle.auth@gmail.com"
credentials_file = "/etc/ple/secrets/gmail-oauth.json"
```

The credential file may contain the OAuth client information, refresh token, and stable Google
subject required by the backend. Its strict serialized format is owned by the Gmail adapter.

The credential remains outside:

- Git and repository history;
- normal configuration examples containing real values;
- environment variables and command arguments;
- container images and build layers;
- application logs and diagnostic output;
- browser-visible configuration;
- PostgreSQL and routine database dumps;
- normal data exports; and
- generated runtime manifests.

PLE does not add application-level encryption whose decryption key would be stored beside the
credential. Exact file ownership, host access controls, and encrypted host storage provide the
initial boundary. A deployment with a real secret manager may later supply the credential through a
different storage adapter.

## Container deployment

In the local Podman topology, a one-shot initializer copies the validated host credential into a
dedicated private runtime volume. Only the API mounts that volume, read-only. The Gmail credential
must not use a shared secret volume also mounted by the worker.

The worker, renderer, gateway, database, object store, installation-data tools, and browser receive
no Gmail credential. The API grants the Gmail adapter outbound TLS access only to Google's fixed
OAuth, OIDC, and Gmail API endpoints (ASVS 12.3.1-12.3.2, 13.2.1-13.2.6).

## Credential lifecycle

The Gmail credential persists across application restarts.

### Initial authorization

The deployment operator authorizes the dedicated sender and PLE writes the protected credential.

### Access-token refresh

The Gmail backend obtains short-lived access tokens using the stored refresh token. This is
ordinary runtime behavior and does not require operator action. Access tokens remain in process
memory and concurrent sends share one bounded refresh operation.

### Reauthorization

If Google revokes the refresh token, the dedicated account changes, or the credential becomes
unusable, the operator runs `reauthorize`. Reauthorization replaces the Gmail installation
credential without changing PLE Accounts or Authenticated Sessions.

Google documents several ways a refresh token may stop working, including revocation, long
inactivity, account password changes, token-count limits, and provider policy. The operator runs
`verify` and `send-test` before each term and after changing the dedicated account. See Google's
[refresh-token expiration rules](https://developers.google.com/identity/protocols/oauth2#expiration).

### Credential replacement

Credential writes use a private temporary file in the destination directory, flush the complete
candidate, validate it, and atomically rename it into place. A partial file never becomes the active
credential.

### Removal

Disabling Gmail delivery does not delete PLE Accounts, authentication records, or Student Work.
The operator removes the local credential and revokes Google authorization through documented
operational steps.

## Runtime behavior

At startup, the Gmail backend validates enough local configuration to determine whether it can
operate. A selected Gmail backend with a missing, unreadable, unsafe, or malformed credential fails
clearly instead of advertising email authentication as available.

Runtime sending:

1. Receives the provider-neutral delivery request.
2. Obtains or refreshes a short-lived Google access token as needed.
3. Constructs one RFC 5322 plain-text message with typed headers.
4. Base64url-encodes the message into the Gmail API `raw` field.
5. Submits it through `users.messages.send`.
6. Returns a provider-neutral receipt or failure category.

The adapter uses fixed HTTPS Google endpoints, ordinary public certificate validation, bounded
request and response sizes, explicit timeouts, limited concurrency, and no redirects. See Google's
[Gmail sending guide](https://developers.google.com/workspace/gmail/api/guides/sending).

A Google outage makes new email authentication unavailable but does not invalidate existing
Authenticated Sessions or stop unrelated PLE teaching capabilities.

## Failure behavior

Important failure classes include:

- Gmail authorization revoked or invalid;
- sender or granted-scope mismatch;
- Gmail account suspended or unavailable;
- Gmail API unavailable;
- sending quota reached;
- network failure;
- invalid local credential;
- institutional rejection or spam filtering; and
- provider acceptance followed by delayed delivery.

PLE distinguishes a definitive rejection from an indeterminate failure after request transmission.
It does not automatically retry an indeterminate send because Gmail may already have accepted the
message. A later permitted authentication start creates a new challenge.

The user-facing authentication flow does not expose Account existence, OAuth credentials, raw
provider errors, internal paths, or Google implementation details. It never creates a session after
a failed or unconsumed challenge (ASVS 6.3.4, 16.5.1-16.5.4).

## Rate limits and capacity

PLE rate-limits authentication starts by browser binding, normalized email, network prefix, and
installation. It separately limits failed completion attempts. Server-owned defaults must permit a
50-Student class to begin sign-in together while preventing one address or client from consuming
the installation's Gmail capacity.

Rate limits never deactivate an Account or permanently lock out a Student. The public response
remains non-enumerating when a hidden email-specific limit applies.

The operator checks Google's current
[Gmail sending limits](https://support.google.com/mail/answer/22839) and
[Gmail API quotas](https://developers.google.com/workspace/gmail/api/reference/quota) during initial
deployment and after a provider-policy notice. PLE does not add speculative scaling infrastructure
for volumes the installation does not have.

## Logging and diagnostics

Operational logging makes failures diagnosable without logging secrets, addresses, or
authentication codes.

Useful information includes:

- selected Email Delivery Backend;
- operation and UTC time;
- provider-neutral success or failure classification;
- safe Gmail API status category;
- request correlation identifier; and
- most recent successful verification or accepted submission time.

Logs exclude OAuth access tokens, refresh tokens, client secrets, authorization codes, provider
response bodies, message bodies, authentication links and codes, browser bindings, session
credentials, raw recipient addresses, and private file paths (ASVS 16.2.1-16.2.5,
16.3.1-16.3.4).

Gmail acceptance and PLE authentication success are separate events. A successful send never
records a successful sign-in, and a provider failure never changes Account State.

## Deliverability

For a small installation, Gmail capacity is expected to be an operational limit rather than Gmail
API request throughput. Institutional spam filtering is the more immediate deployment concern.

Before relying on Gmail for authentication, the deployment operator verifies delivery to several
real `mail.roosevelt.edu` accounts, including practical latency and spam-folder behavior. This is an
attended deployment check; PLE does not request mailbox-read access to automate it.

A free Gmail sender that remains reliable and within applicable limits may continue to be used. The
design does not describe Gmail as temporary merely because another provider may eventually become
desirable.

## Provider replacement

The Email Delivery Backend is the migration boundary.

A future installation may use SMTP, Google Workspace, or another provider. Changing the provider
replaces backend configuration and credentials without changing:

- PLE Account identity;
- Product Roles;
- institutional-domain rules;
- authentication-challenge semantics;
- Authenticated Sessions;
- enrollment; or
- authorization.

Provider-specific configuration remains inside its adapter. PLE builds another adapter only when an
actual deployment need justifies it.

## Web interface

PLE does not add a dedicated Gmail setup webpage for the initial design.

OAuth setup is an infrequent installation operation involving privileged deployment credentials. A
permanent web surface would add routes, authorization rules, credential handling, callback
behavior, interface maintenance, and attack surface for work normally performed once.

A future read-only Sysadmin status view may expose `configured`, `available`, and a generic
last-check time if this becomes useful during normal operation. It does not expose authorization,
credential upload, reauthorization, test delivery, host paths, or raw provider errors.

A web setup workflow is reconsidered only if provider configuration becomes a routine Sysadmin
task.

## Security boundary

The Gmail refresh token is a deployment credential with the authority granted by its OAuth scope.
Security depends on:

- requesting only the required Google authorization;
- validating the exact authorized sender and granted scopes;
- protecting the host credential file;
- limiting runtime access to the API Gmail adapter;
- fixing trusted Google endpoints in code;
- excluding secrets from logs, exports, database state, and browser responses;
- making reauthorization straightforward; and
- revoking Google authorization when the backend is retired.

Compromise of the PLE host can expose a credential available to the PLE process. The initial design
does not claim to solve host compromise through an application-specific encryption layer.

Email is not a high-assurance authenticator under current NIST and ASVS guidance. PLE accepts that
risk for the required teaching workflow, limits authentication to configured institutional
mailboxes, binds and expires each challenge, applies abuse controls, and retains passkeys as the
stronger optional path. Email authentication alone does not satisfy an ASVS Level 2 multi-factor
requirement or an ASVS Level 3 authentication target.

## Operational recovery

Recovery remains short enough to perform without application surgery:

```text
1. Confirm the dedicated Gmail account is accessible.
2. Confirm the Google Cloud Gmail API configuration remains valid.
3. Run the Gmail reauthorization command.
4. Complete Google authorization.
5. Run the remote verification command.
6. Send a test message to a university address.
7. Resume email authentication.
```

No Account migration, database edit, session redesign, or Student action is required.

## Backups

The Gmail credential is installation configuration rather than educational data.

Deployment documentation states whether private installation credentials are included in encrypted,
access-controlled host backups. If they are backed up, the backup receives the same sensitivity as
the original credential. If they are not backed up, Gmail reauthorization is the recovery method.

PostgreSQL backups do not contain Gmail credentials. Losing the Gmail credential disables new email
delivery but does not lose PLE Accounts, Course state, or Student Work.

## Deployment evolution

The first deployment may use a free consumer Gmail account to minimize cost. This remains
acceptable while actual sending volume, provider policy, reliability, account ownership, and
institutional deliverability remain suitable.

Migration is driven by observed operational needs. Reasons to move may include:

- repeated deliverability problems;
- sending limits that constrain real usage;
- account-management or institutional requirements;
- a need for stronger organizational ownership;
- provider reliability problems; or
- a desire to send from a PLE-owned or institution-owned domain.

The provider-neutral boundary keeps that migration bounded when it becomes justified.

## Acceptance criteria

The Gmail backend is ready for an installation when:

1. A deployment operator can authorize a dedicated Gmail sender using documented PLE operator
   tooling.
2. The resulting OAuth credential is stored outside the repository with host-only access.
3. PLE can restart and continue sending without interactive Google authorization.
4. The backend requests only `gmail.send` plus the OIDC identity claims needed to verify the
   configured sender.
5. An operator can inspect local and remote backend status without exposing credential values.
6. An operator can send one explicit test message.
7. A real authentication message reaches representative permitted university mailboxes within an
   acceptable time.
8. The same requesting browser can complete the PLE challenge exactly once, while reuse, expiry,
   another browser, and incorrect challenges fail.
9. Authentication-start responses do not reveal Account existence or eligibility.
10. Gmail-specific implementation details remain behind the Email Delivery Backend.
11. Gmail failure produces useful diagnostics without leaking credentials, addresses, or
    authentication challenges.
12. Reauthorizing, disabling, or replacing Gmail does not alter PLE Accounts, Product Roles,
    Authenticated Sessions, enrollment, or Student Work.
13. Existing Authenticated Sessions and unrelated PLE capabilities remain available during a Gmail
    outage.
14. The design permits a later provider to replace Gmail without redesigning PLE authentication.

## Verification evidence

Permanent offline tests cover strict configuration parsing, private-file validation, secret
redaction, OAuth validation, typed message construction, fixed endpoints, timeout and retry
behavior, non-enumerating responses, rate limits, browser binding, single consumption, and session
issuance.

A deterministic fake Email Delivery Backend may exercise the complete browser flow. That evidence
does not claim Gmail delivery.

Connected acceptance uses the dedicated Gmail sender and representative real university mailboxes.
It records only redacted outcomes and delivery times. Gmail submission, mailbox receipt, challenge
completion, and Authenticated Session creation are distinct evidence claims under
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

## Security requirements applied

This specification applies the following ASVS controls:

| ASVS requirements | Application |
| --- | --- |
| 1.1.1-1.1.2, 1.2.1-1.2.5 | Canonical email input, typed mail headers, and final-context encoding |
| 2.1.1-2.1.3, 2.3.1-2.3.4 | Strict credential validation and ordered, atomic credential lifecycle |
| 3.3.1-3.3.4, 6.3.1-6.3.8, 6.6.2-6.6.3 | Host-only browser credentials, non-enumeration, challenge binding, expiry, rate limits, and replay resistance |
| 8.2.1-8.2.3, 8.3.1 | Gmail delivery grants no PLE identity or authorization |
| 9.1.1-9.1.3, 10.5.1-10.5.4 | Fixed-issuer OAuth validation and minimum-scope backend tokens |
| 11.4.1-11.4.3, 11.5.1 | Approved hashes, keyed rate-limit identifiers, and operating-system randomness |
| 12.3.1-12.3.2, 13.2.1-13.2.6 | Authenticated TLS, fixed external systems, timeouts, bounded concurrency, and controlled retries |
| 14.2.1-14.2.7, 16.1.1, 16.2.1-16.5.4 | Least-authority secret storage, process isolation, redacted audit events, and secure failure behavior |
