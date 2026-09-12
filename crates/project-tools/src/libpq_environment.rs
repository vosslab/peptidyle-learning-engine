//! Validated libpq variables for short-lived PostgreSQL client processes.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail, ensure};
use percent_encoding::percent_decode_str;
use url::Url;

/// A validated child-process environment derived from a PostgreSQL URL.
///
/// The connection URL remains in the parent process. Child processes receive
/// only the libpq variables they need, so no secret reaches argv or generated
/// SQL. This supports ASVS 1.2.5 and 2.2.1.
pub(crate) struct LibpqEnvironment {
    variables: BTreeMap<String, String>,
}

impl LibpqEnvironment {
    pub(crate) fn from_database_url(database_url: &str, url_context: &str) -> Result<Self> {
        let parsed =
            Url::parse(database_url).map_err(|_| anyhow::anyhow!("{url_context} is not valid"))?;
        ensure!(
            matches!(parsed.scheme(), "postgres" | "postgresql"),
            "{url_context} must use PostgreSQL"
        );
        ensure!(parsed.fragment().is_none(), "{url_context} is not valid");
        let host = parsed
            .host_str()
            .with_context(|| format!("{url_context} must include a host"))?;
        let user = decode_url_component(parsed.username(), url_context)?;
        let password = parsed
            .password()
            .filter(|value| !value.is_empty())
            .with_context(|| format!("{url_context} must include a password"))
            .and_then(|value| decode_url_component(value, url_context))?;
        let database = decode_url_component(
            parsed.path().strip_prefix('/').unwrap_or_default(),
            url_context,
        )?;
        ensure!(
            !user.is_empty() && !database.is_empty() && !database.contains('/'),
            "{url_context} is not valid"
        );
        for value in [host, user.as_str(), password.as_str(), database.as_str()] {
            ensure!(
                !value.contains(['\0', '\n', '\r']),
                "{url_context} is not valid"
            );
        }

        let mut variables = BTreeMap::from([
            ("PGHOST".to_string(), host.to_string()),
            (
                "PGPORT".to_string(),
                parsed.port().unwrap_or(5432).to_string(),
            ),
            ("PGUSER".to_string(), user),
            ("PGDATABASE".to_string(), database),
            ("PGPASSWORD".to_string(), password),
        ]);
        let allowed_options = [
            ("sslmode", "PGSSLMODE"),
            ("sslrootcert", "PGSSLROOTCERT"),
            ("sslcert", "PGSSLCERT"),
            ("sslkey", "PGSSLKEY"),
            ("sslcrl", "PGSSLCRL"),
            ("sslcrldir", "PGSSLCRLDIR"),
            ("sslpassword", "PGSSLPASSWORD"),
            ("sslnegotiation", "PGSSLNEGOTIATION"),
            ("gssencmode", "PGGSSENCMODE"),
        ];
        for (key, value) in parsed.query_pairs() {
            let Some((_, environment_key)) = allowed_options
                .iter()
                .find(|(allowed_key, _)| *allowed_key == key)
            else {
                bail!("{url_context} contains an unsupported connection option");
            };
            ensure!(
                !value.is_empty()
                    && !value.contains(['\0', '\n', '\r'])
                    && !variables.contains_key(*environment_key),
                "{url_context} is not valid"
            );
            variables.insert((*environment_key).to_string(), value.into_owned());
        }
        Ok(Self { variables })
    }

    pub(crate) fn variables(&self) -> &BTreeMap<String, String> {
        &self.variables
    }
}

fn decode_url_component(value: &str, url_context: &str) -> Result<String> {
    percent_decode_str(value)
        .decode_utf8()
        .map(|value| value.into_owned())
        .map_err(|_| anyhow::anyhow!("{url_context} is not valid"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_environment_has_required_postgres_and_tls_values_without_a_url() {
        let environment = LibpqEnvironment::from_database_url(
            "postgresql://ple_migrator:secret@database.example:5444/ple?sslmode=verify-full&sslrootcert=%2Frun%2Fsecrets%2Froot.crt",
            "database administration URL",
        )
        .unwrap();
        assert_eq!(
            environment.variables().get("PGHOST"),
            Some(&"database.example".to_string())
        );
        assert_eq!(
            environment.variables().get("PGPORT"),
            Some(&"5444".to_string())
        );
        assert_eq!(
            environment.variables().get("PGUSER"),
            Some(&"ple_migrator".to_string())
        );
        assert_eq!(
            environment.variables().get("PGDATABASE"),
            Some(&"ple".to_string())
        );
        assert_eq!(
            environment.variables().get("PGSSLMODE"),
            Some(&"verify-full".to_string())
        );
        assert_eq!(
            environment.variables().get("PGSSLROOTCERT"),
            Some(&"/run/secrets/root.crt".to_string())
        );
        assert!(!environment.variables().contains_key("DATABASE_URL"));
        assert!(
            !environment
                .variables()
                .keys()
                .any(|key| key.contains("URL"))
        );
        assert!(
            !environment
                .variables()
                .values()
                .any(|value| value.contains("://"))
        );
    }

    #[test]
    fn child_environment_rejects_unsupported_or_duplicate_connection_options() {
        for url in [
            "postgres://u:p@host/db?application_name=untrusted",
            "postgres://u:p@host/db?sslmode=require&sslmode=disable",
        ] {
            assert!(
                LibpqEnvironment::from_database_url(url, "installation database URL").is_err(),
                "{url}"
            );
        }
    }
}
