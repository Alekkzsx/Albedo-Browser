use super::{parse, stringify, JsonValue};

#[test]
fn test_real_world_github_api() {
    let json = r#"{
        "id": 1296269,
        "node_id": "MDEwOlJlcG9zaXRvcnkxMjk2MjY5",
        "name": "Hello-World",
        "full_name": "octocat/Hello-World",
        "private": false,
        "owner": {
            "login": "octocat",
            "id": 1,
            "avatar_url": "https://github.com/images/error/octocat_happy.gif",
            "type": "User",
            "site_admin": false
        },
        "html_url": "https://github.com/octocat/Hello-World",
        "description": "This your first repo!",
        "fork": false,
        "url": "https://api.github.com/repos/octocat/Hello-World",
        "created_at": "2011-01-26T19:01:12Z",
        "updated_at": "2011-01-26T19:01:12Z",
        "pushed_at": "2011-01-26T19:01:12Z",
        "git_url": "git://github.com/octocat/Hello-World.git",
        "ssh_url": "git@github.com:octocat/Hello-World.git",
        "clone_url": "https://github.com/octocat/Hello-World.git",
        "svn_url": "https://github.com/octocat/Hello-World",
        "homepage": "https://github.com",
        "size": 108,
        "stargazers_count": 80,
        "watchers_count": 80,
        "language": null,
        "has_issues": true,
        "has_projects": true,
        "has_downloads": true,
        "has_wiki": true,
        "has_pages": false,
        "forks_count": 9,
        "mirror_url": null,
        "archived": false,
        "disabled": false,
        "open_issues_count": 0,
        "license": {
            "key": "mit",
            "name": "MIT License",
            "spdx_id": "MIT",
            "url": "https://api.github.com/licenses/mit",
            "node_id": "MDc6TGljZW5zZTEz"
        },
        "allow_forking": true,
        "is_template": false,
        "topics": [
            "octocat",
            "atom",
            "electron",
            "api"
        ],
        "visibility": "public",
        "forks": 9,
        "open_issues": 0,
        "watchers": 80,
        "default_branch": "master"
    }"#;

    let parsed = parse(json).expect("Deve parsear JSON do GitHub");
    assert_eq!(
        parsed.get("name").and_then(|v| v.as_string()),
        Some("Hello-World")
    );
    assert_eq!(
        parsed
            .get("owner")
            .and_then(|v| v.get("login"))
            .and_then(|v| v.as_string()),
        Some("octocat")
    );
    assert_eq!(parsed.get("private").and_then(|v| v.as_bool()), Some(false));
    assert!(parsed.get("language").unwrap().is_null());

    let topics = parsed.get("topics").and_then(|v| v.as_array()).unwrap();
    assert_eq!(topics.len(), 4);
    assert_eq!(topics[0].as_string(), Some("octocat"));

    // Round-trip
    let serialized = stringify(&parsed);
    let parsed2 = parse(&serialized).expect("Deve parsear novamente");
    assert_eq!(parsed, parsed2);

    let pretty = super::stringify_pretty(&parsed);
    assert!(pretty.contains("\n"));
}

#[test]
fn test_import_map_complex() {
    let json = r#"{
        "imports": {
            "react": "https://cdn.esm.sh/react@18",
            "react-dom": "https://cdn.esm.sh/react-dom@18",
            "./utils/": "./src/utils/"
        },
        "scopes": {
            "https://cdn.esm.sh/": {
                "react": "https://cdn.esm.sh/react@17"
            }
        }
    }"#;

    let parsed = parse(json).expect("Deve parsear Import Map");
    let imports = parsed.get("imports").and_then(|v| v.as_object()).unwrap();
    assert_eq!(
        imports.get("react").and_then(|v| v.as_string()),
        Some("https://cdn.esm.sh/react@18")
    );

    let scopes = parsed.get("scopes").and_then(|v| v.as_object()).unwrap();
    let esm_scope = scopes
        .get("https://cdn.esm.sh/")
        .and_then(|v| v.as_object())
        .unwrap();
    assert_eq!(
        esm_scope.get("react").and_then(|v| v.as_string()),
        Some("https://cdn.esm.sh/react@17")
    );
}

#[test]
fn test_edge_cases_escapes() {
    let json = r#""Line 1\nLine 2\tTab\rCarriage\\Backslash\"Quote\/\u0041""#;
    let parsed = parse(json).expect("Deve parsear escapes");
    assert_eq!(
        parsed.as_string(),
        Some("Line 1\nLine 2\tTab\rCarriage\\Backslash\"Quote/A")
    );
}

#[test]
fn test_numbers_extreme() {
    let json = "[1.23e+10, -0.456, 1000, 0]";
    let parsed = parse(json).expect("Deve parsear números");
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr[0].as_number(), Some(12300000000.0));
    assert_eq!(arr[1].as_number(), Some(-0.456));
    assert_eq!(arr[2].as_number(), Some(1000.0));
    assert_eq!(arr[3].as_number(), Some(0.0));
}

#[test]
fn test_trailing_trash_rejection() {
    let json = r#"{"a": 1} abc"#;
    let result = parse(json);
    assert!(result.is_err());
}
