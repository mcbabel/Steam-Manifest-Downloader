//! Secrets that a download config carries for one run but that must never be
//! written to disk with it (history resume payloads, the download queue).

/// Config keys (camelCase, as `DownloadConfig` serializes them) holding a
/// secret. The frontends fill them in again from their own storage when they
/// resume or start a stored download.
const SECRET_KEYS: &[&str] = &["manifestHubApiKey", "branchPassword"];

/// Removes the secret fields from a serialized `DownloadConfig`. Returns
/// whether anything was removed.
pub fn strip(config: &mut serde_json::Value) -> bool {
    let Some(map) = config.as_object_mut() else {
        return false;
    };
    let mut removed = false;
    for key in SECRET_KEYS {
        removed |= map.remove(*key).is_some();
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn removes_the_manifesthub_key_and_keeps_the_rest() {
        let mut v =
            json!({ "mainAppId": "480", "manifestHubApiKey": "secret", "selectedDepots": [] });
        assert!(strip(&mut v));
        assert_eq!(v, json!({ "mainAppId": "480", "selectedDepots": [] }));
        assert!(!strip(&mut v));
    }

    #[test]
    fn removes_the_branch_password() {
        let mut v = json!({ "mainAppId": "480", "branch": "beta", "branchPassword": "pw", "selectedDepots": [] });
        assert!(strip(&mut v));
        assert_eq!(
            v,
            json!({ "mainAppId": "480", "branch": "beta", "selectedDepots": [] })
        );
    }

    #[test]
    fn a_branch_or_the_flag_means_the_account_is_used() {
        let parse = |v: serde_json::Value| -> crate::ops::download::DownloadConfig {
            serde_json::from_value(v).unwrap()
        };
        assert!(!parse(json!({ "mainAppId": "1", "selectedDepots": [] })).uses_account());
        assert!(
            !parse(json!({ "mainAppId": "1", "selectedDepots": [], "branch": "Public" }))
                .uses_account()
        );
        assert!(
            parse(json!({ "mainAppId": "1", "selectedDepots": [], "branch": "beta" }))
                .uses_account()
        );
        assert!(
            parse(json!({ "mainAppId": "1", "selectedDepots": [], "useAccount": true }))
                .uses_account()
        );
    }

    #[test]
    fn leaves_non_objects_alone() {
        let mut v = json!(null);
        assert!(!strip(&mut v));
    }

    #[test]
    fn a_stripped_snapshot_still_parses_as_a_download_config() {
        let mut v =
            json!({ "mainAppId": "480", "manifestHubApiKey": "secret", "selectedDepots": [] });
        strip(&mut v);
        let config: crate::ops::download::DownloadConfig = serde_json::from_value(v).unwrap();
        assert!(config.manifest_hub_api_key.is_none());
    }
}
