# Gmail API Email Backend Specification

## Status

Proposed specification for an optional Gmail API Email Delivery Backend running in a dedicated
Podman mail-sender container. The initial integration targets the local Podman installation.
The selected credential design uses a portable encrypted file and an operator-entered passphrase
at mail-sender startup, supporting both macOS and Debian without requiring Keychain.

PLE already implements SMTP-backed, browser-bound email-code authentication and
Instructor setup notices. An external provider configuration and real-inbox
acceptance remain pending. This document does not claim that the Gmail API
adapter, mail-sender service, OAuth setup commands, or Gmail connected-delivery acceptance exist
today. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) remains the product authority,
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns PLE vocabulary, and
[ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md) owns Account, Course enrollment,
roster import, and Course Invitation behavior.

The Gmail API backend is intended to support small PLE installations without requiring a paid
email-delivery service. It may begin as the backend for a Pilot deployment, but its design is
suitable for continued production use while message volume, provider policy, reliability, and
deliverability remain acceptable.

Gmail is an email transport provider only. Gmail does not own PLE Accounts, authentication
decisions, User Roles, Authenticated Sessions, enrollment, or authorization.

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
Student Authentication Email assignment remain part of Course enrollment. Instructor
Instructor Account creation follows outside-PLE vetting and the bounded
Sysadmin account-creation operation.

## Architectural boundary

PLE authentication owns the authentication workflow. The Email Delivery Backend only delivers a
bounded message to an address selected by PLE. A dedicated mail-sender service owns Google
authorization, token refresh, message construction, and submission. The PLE API holds no Google
credential.

```text
PostgreSQL <--> PLE API
                  |
                  | authenticated, bounded delivery request
                  v
             Mail-sender container <--- protected runtime credential
                  |
                  | HTTPS
                  v
               Gmail API
```

The mail-sender service receives a validated recipient, a supported message kind, and the bounded
values required by that kind, such as a sign-in code. The service owns the sender, subjects,
plain-text templates, and configured sign-in URL. It does not decide whether the recipient may
authenticate, create or validate challenges, create sessions, determine User Roles, inspect Course
Membership, or authorize Student Work. It has no database connection or database-polled mail queue.

No Gmail-specific concept appears in the Account, authentication-challenge, or session model.
Provider replacement changes installation configuration and credentials, not product identity.

## Provider-neutral delivery contract

The server owns a private client for one narrow operation equivalent to:

```rust
pub trait EmailDeliveryBackend {
    async fn send(
        &self,
        message: EmailDeliveryMessage,
    ) -> Result<EmailDeliveryReceipt, EmailDeliveryFailure>;
}
```

`EmailDeliveryMessage` is a closed application type. The initial message kinds cover sign-in codes,
Instructor setup notices, and explicit operator test messages. Each kind accepts only its required
typed values. The service rejects unknown kinds, unknown fields, invalid recipients, oversized
requests, and values outside the selected kind's limits before contacting Google.

Callers supply no arbitrary subject, body, template, sender, link, provider URL, OAuth scope, mail
header, attachment, or credential. The mail-sender service constructs each message from its own
fixed template and trusted installation configuration. Its response contains only a bounded receipt
or failure category, with no credential-reading or general debugging operation.

`EmailDeliveryReceipt` means that the provider accepted the submission. It does not prove inbox
delivery or successful authentication.

Provider-specific errors remain inside the mail-sender service. The service returns bounded
operational categories such as configuration, credential, quota, recipient, transient,
indeterminate, and protocol failure. Provider response bodies and OAuth error descriptions never
enter browser responses or ordinary logs.

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
11. PLE derives authorization and User Role from the PLE Account and exact product
    relationships.

Receiving a message at a university address proves control of that mailbox for the limited purpose
defined by PLE. It does not make Google an identity provider.

Students do not need Gmail accounts and do not authorize PLE to access Google. A Student may
authenticate as `student@mail.roosevelt.edu` while the message is sent from a dedicated
`@gmail.com` account.

Course enrollment, including roster import, permits exact configured domains such as
`mail.roosevelt.edu`. A suffix such as `.edu` establishes the United States institutional-address
eligibility rule but is not Course authorization. Student email is immutable after Account
creation. The public authentication-start response must not reveal whether an address belongs to
an Account, is inactive, is outside the permitted domain, or is currently rate-limited (ASVS
6.3.1, 6.6.2-6.6.3).

## Authentication-message content

Authentication messages contain no Student name, Course name, roster ID, Account identifier,
User Role, grade, Assessment, or Student Work. A message contains only the installation name,
authentication link or code, expiration information, and instructions to ignore an unrequested
message.

The initial backend sends plain text without attachments, tracking pixels, remote images,
marketing content, `Cc`, or `Bcc`. Message construction uses typed header setters so recipient data
cannot inject a mail header (ASVS 1.1.1-1.1.2, 1.2.1-1.2.5).

## Sender account

Each installation using the Gmail backend uses a dedicated Gmail account. The selected sender for
this project is:

```text
peptidyle.admin@gmail.com
```

The exact address is installation configuration. The word `admin` in the address grants no PLE or
Google Cloud administrative authority. Creating a second administration mailbox is not required
for sending email.

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
access tokens as required by Google. Runtime access and refresh tokens remain inside the
mail-sender service. They never enter the PLE API, browser state, application data, or product
authorization (ASVS 9.1.1-9.1.3, 10.5.1-10.5.4).

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
11. Choose a separate local passphrase and encrypt the resulting installation credential outside
    the repository.
12. Configure the installation to use the Gmail API backend.
13. Start the mail-sender and unlock its credential with the local passphrase.
14. Send a test message to real university destinations.

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
Sysadmin Account, but the Sysadmin User Role alone grants no host or Gmail credential authority.

### Authorize and reauthorize

`authorize` performs the interactive OAuth exchange for the dedicated sender account. It:

- reads the downloaded Google OAuth client configuration from an explicit file argument;
- starts Google's supported Desktop application authorization-code flow;
- listens temporarily on a random `127.0.0.1` loopback port;
- validates OAuth state, PKCE, OIDC nonce, issuer, audience, expiry, subject, verified email, and
  granted scopes;
- verifies the authorized email exactly matches the configured sender;
- prompts without echo for a separate local passphrase and writes the encrypted installation
  credential to the protected file;
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
Sender: peptidyle.admin@gmail.com
Local credential: encrypted envelope has valid shape
```

Local inspection checks file permissions and the encrypted envelope's shape without decrypting its
contents. It does not establish that the passphrase, enclosed credential, or Google grant is valid.

`verify` contacts Google, refreshes an access token, and confirms the authorized sender and granted
scope without sending mail. It distinguishes missing configuration, missing or unreadable files,
invalid credential format, revoked authorization, sender mismatch, scope mismatch, and unavailable
Google service. `verify` and `send-test` use the unlocked mail-sender; a locked sender reports that
operator unlocking is required.

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
ordinary application data. The local deployment uses this default on both macOS and Debian:

```text
~/.local/share/ple/
    gmail-oauth.enc
```

The encrypted file belongs to the deployment operator and persists independently of the disposable
Live Demo workspace. Demo reset, reseeding, cleanup, and image rebuilding preserve it. Other
deployments may select another protected location outside the checkout.

The secret directory is owned by the deployment account, is not a symbolic link, and uses mode
`0700` where supported. The credential is a regular non-symbolic-link file owned by the deployment
account with exact mode `0600`. PLE bounds the file size, strictly validates its serialized shape,
and refuses an unsafe, unreadable, or malformed credential (ASVS 2.1.1-2.1.3, 14.2.1-14.2.7).

Normal configuration stores only non-secret information and the credential location. Conceptually:

```toml
[email]
backend = "gmail_api"
sender = "peptidyle.admin@gmail.com"
credentials_file = "/run/ple/secrets/gmail-oauth.enc"
```

The example names the sender's private container path. The deployment mounts only the encrypted
host file there, read-only; it does not mount the operator's home or credential directory.

These settings belong to the mail-sender service, not the PLE API. The PLE API receives only the
private service endpoint and the deployment-managed caller identity needed to submit messages.
That identity remains outside PostgreSQL and grants no access to Google tokens.

The encrypted payload contains the OAuth client information, refresh token, and stable Google
subject required by the service. Its strict serialized format is owned by the Gmail adapter.
Access tokens remain in memory.

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

Persistent credentials and their backups contain ciphertext. Use the maintained RustCrypto
[Argon2id key-derivation API](https://docs.rs/argon2/latest/argon2/#key-derivation) to derive the
encryption key from the separate operator passphrase and a random salt. Encrypt with the project's
existing [XChaCha20Poly1305 library](https://docs.rs/chacha20poly1305/latest/chacha20poly1305/),
using a fresh random nonce for each write. Authenticated encryption detects modification as well
as protecting confidentiality (ASVS 11.2.1, 11.3.3-11.3.4).

Use a long, unique passphrase separate from Gmail, operating-system, and PLE Sysadmin passwords. Store
neither the passphrase nor the derived key beside the encrypted file or in saved configuration,
arguments, environment variables, logs, or persistent Podman secrets. File permissions and a
`sudo` prompt control access but do not encrypt copied files or backups. The selected design
requires no macOS Keychain, GPG installation, or host administrator privileges.

Only the dedicated sender receives runtime unlock material and decrypted OAuth credentials through
the protected startup handoff. Keep them in process memory, clear transient secret buffers when no
longer needed, and write no persistent plaintext copy. Verify the handoff and Podman VM storage
before claiming this boundary is implemented (ASVS 13.3.1-13.3.2, 14.2.1-14.2.7).

## Container deployment

The mail-sender image contains code and dependencies. Image builds require no OAuth credential or
passphrase. After building, the deployment mounts the encrypted file only into the mail-sender
container and supplies unlock material through a protected runtime handoff. Credentials and
passphrases stay out of image layers, build contexts, registries, and container environment
variables. Podman supports
[runtime secret mounts](https://docs.podman.io/en/latest/markdown/podman-run.1.html#secret-secret-opt-opt);
using a secret mount does not itself establish encryption of its backing storage. Persistent
Podman secret storage holds no plaintext Gmail credential, passphrase, or derived key.

The mail-sender runs as a non-root user with a read-only root filesystem, dropped capabilities,
and no Podman control socket or broad host-directory mount. The PLE API, workers, renderer,
gateway, database, object store, installation-data tools, and browser receive no Gmail credential
or access to its storage. The trusted host authorization command provisions credentials during
installation and recovery.

Only the PLE API and attended operator tools may submit supported requests over the private,
authenticated service connection. The sender's endpoint has no public host-port mapping. The
database does not join the sender's private network or receive its caller identity. Endpoint,
template, sender, and service-authorization configuration remain outside PostgreSQL. Private
network placement and strict request parsing complement caller authentication; they do not replace
it (ASVS 8.3.1, 13.2.1-13.2.2).

The mail-sender receives outbound TLS access only to Google's fixed OAuth, OIDC, and Gmail API
endpoints. It has no database login or access to PLE object storage. Its independent sending limits
remain outside database control (ASVS 12.3.1-12.3.2, 13.2.1-13.2.6).

## Credential lifecycle

The encrypted Gmail credential persists across application restarts and disposable demo resets.

### Initial authorization

The deployment operator completes Google consent for the dedicated sender, chooses a separate
local passphrase, and PLE writes the encrypted credential. Google authorization and local
encryption are separate operations.

### Startup and unlocking

After image builds finish, starting the mail-sender requests the local passphrase through a
terminal prompt with echo disabled. The sender decrypts and validates the credential in memory
before becoming ready to send. A cancelled prompt, wrong passphrase, or failed integrity check
preserves the encrypted file and leaves email delivery unavailable.

The unlock lasts for the sender process's lifetime. Restarting only the PLE API or refreshing the
browser does not require another unlock while the sender remains running. Stopping or restarting
the sender loses the unlock; unattended restarts remain locked until the operator supplies the
passphrase again. Routine startup does not repeat Google consent.

### Access-token refresh

The mail-sender service obtains short-lived access tokens using the stored refresh token. This is
ordinary runtime behavior after local unlocking and does not require operator action per message
or token refresh. Access tokens remain in process memory and concurrent sends share one bounded
refresh operation.

### Reauthorization

If Google revokes the refresh token, the dedicated account changes, or the credential becomes
unusable, the operator runs `reauthorize`. Reauthorization replaces the Gmail installation
credential without changing PLE Accounts or Authenticated Sessions.

For a forgotten local passphrase, reauthorize with Google and encrypt the replacement credential
under a new passphrase. Recovery requires no old passphrase or database changes. Restart and
unlock the sender after replacement; failed reauthorization preserves the previous encrypted file.

Google documents several ways a refresh token may stop working, including revocation, long
inactivity, account password changes, token-count limits, and provider policy. The operator runs
`verify` and `send-test` before each term and after changing the dedicated account. See Google's
[refresh-token expiration rules](https://developers.google.com/identity/protocols/oauth2#expiration).

### Credential replacement

Credential writes encrypt in memory, use a private ciphertext-only temporary file in the
destination directory, flush and validate the complete candidate, and atomically rename it into
place. A partial file never becomes the active credential, and plaintext is never staged on disk.

### Removal

Disabling Gmail delivery does not delete PLE Accounts, authentication records, or Student Work.
The operator removes the local credential and revokes Google authorization through documented
operational steps.

## Runtime behavior

At startup, the Gmail backend validates enough local configuration to determine whether it can
operate. A selected Gmail backend with a missing, unreadable, unsafe, malformed, or locked
credential fails clearly instead of advertising email authentication as available.

Runtime sending:

1. Authenticates the private caller and validates the selected message kind and its values.
2. Applies independent sending limits, then obtains or refreshes a short-lived Google access token.
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

The mail-sender service also enforces installation-wide and per-recipient sending limits outside
PostgreSQL. It applies these limits to every allowed sender request, including cover messages and
operator tests. A compromised database cannot raise or reset that budget. A valid request format
alone does not authorize unlimited sending. Limits accommodate a 50-Student class sharing a campus
network; this design introduces no general mail queue or bulk-mail workflow.

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
- User Roles;
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
- limiting runtime credential access to the dedicated mail-sender service;
- authenticating private callers and rendering only supported message templates;
- enforcing sending limits independently of PostgreSQL;
- fixing trusted Google endpoints in code;
- excluding secrets from logs, exports, database state, and browser responses;
- making reauthorization straightforward; and
- revoking Google authorization when the backend is retired.

The intended protection depends on which component is compromised:

| Compromised component | Protection and remaining exposure |
| --- | --- |
| Database dump or PostgreSQL container alone | Contains no Gmail token, service caller credential, or secret mount; cannot directly call the sender. Database writes may still change Account or recipient data used by normal PLE workflows. |
| PLE API | Cannot directly read Gmail credentials, but can submit supported requests within the sender's independently enforced limits. |
| Mail-sender process | Can use the Google credential and see messages it handles; send-only OAuth scope limits Google authority. |
| Host or Podman administrator | Can access or control the running sender and its secrets; container isolation does not cover this compromise. |

Strict formatting protects parsing and message construction. It does not establish that an address
read from a compromised database is legitimate, restore Account integrity, or stop an authorized
caller from attempting abusive requests. Credential isolation and application authorization remain
separate responsibilities. Encryption at rest does not prevent a compromised running mail-sender
from using its credential.

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
4. Complete Google authorization and encrypt the credential with a local passphrase.
5. Restart and unlock the mail-sender.
6. Run the remote verification command.
7. Send a test message to a university address.
8. Resume email authentication.
```

No Account migration, database edit, session redesign, or Student action is required.

## Backups

The Gmail credential is installation configuration rather than educational data.

Deployment documentation states whether private installation credentials are included in encrypted,
access-controlled host backups. If they are backed up, the backup receives the same sensitivity as
the original credential and contains only the encrypted file, never the passphrase or derived key.
Restore requires the local passphrase. If the credential is not backed up, or the passphrase is
forgotten, Gmail reauthorization is the recovery method.

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
2. The resulting OAuth credential is encrypted outside the repository with a mode-0700 directory
   and mode-0600 file, using the same credential format on macOS and Debian.
3. After sender restart, the operator unlocks with the local passphrase and sending resumes
   without interactive Google authorization. API restarts leave a running sender unlocked.
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
12. Reauthorizing, disabling, or replacing Gmail does not alter PLE Accounts, User Roles,
    Authenticated Sessions, enrollment, or Student Work.
13. Existing Authenticated Sessions and unrelated PLE capabilities remain available during a Gmail
    outage.
14. The design permits a later provider to replace Gmail without redesigning PLE authentication.
15. Only the mail-sender container receives runtime Gmail credentials. Images, PostgreSQL, the PLE
    API, workers, and other containers contain no Google token or mount of its storage.
16. Unauthorized private callers, unsupported messages, malformed requests, and requests beyond
    the sender's independent limits fail before Google submission.
17. Wrong passphrases, cancelled unlocking, and modified encrypted files preserve the stored
    credential and leave sending unavailable. Image builds require no unlock or OAuth secrets.
18. Demo resets preserve the external encrypted file. Backup restoration with the passphrase and
    Google reauthorization after passphrase loss both restore sending without database changes.

## Verification evidence

Implementation tests must cover strict configuration parsing, private-file validation, secret
redaction, OAuth validation, typed message construction, fixed endpoints, timeout and retry
behavior, non-enumerating responses, rate limits, browser binding, single consumption, and session
issuance.

Credential checks must cover encryption round trips, wrong passphrases, tamper rejection, atomic
replacement, redaction, and the same file format on macOS and Debian. Connected startup checks
must verify the hidden prompt, protected host-to-Podman handoff, sender-restart locking, API-restart
continuity, preservation across demo resets, encrypted backups, and passphrase-loss recovery.
These checks remain required implementation evidence, not results established by this document.

Connected container checks must prove that the API and database cannot read the sender's secret,
the database cannot directly submit mail, unauthorized callers and invalid request shapes are
rejected, and database changes cannot disable sender-owned limits. These checks verify isolation
and bounded sending; they do not establish safety after a host or mail-sender compromise.

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
| 11.2.1, 11.3.3-11.3.4, 13.3.1-13.3.2 | Maintained authenticated encryption, fresh nonces, separate manual unlock, runtime-only secret access, and credential-free builds |
| 11.4.1-11.4.3, 11.5.1 | Approved hashes, keyed rate-limit identifiers, and operating-system randomness |
| 12.3.1-12.3.2, 13.2.1-13.2.6 | Authenticated TLS, fixed external systems, timeouts, bounded concurrency, and controlled retries |
| 14.2.1-14.2.7, 16.1.1, 16.2.1-16.5.4 | Least-authority secret storage, process isolation, redacted audit events, and secure failure behavior |
