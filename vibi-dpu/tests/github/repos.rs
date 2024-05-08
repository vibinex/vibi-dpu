use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;

    #[test]
    fn test_get_user_github_repos_using_graphql_api_success() {
        let mock_access_token = "test_token";
        let mock_repos = vec![RepoIdentifier {
            id: 1,
            name: "test_repo".to_string(),
            owner: "test_owner".to_string(),
        }];

        let mock = mock("GET", "/graphql")
            .with_status(200)
            .with_body(json!({
                "data": {
                    "user": {
                        "repositories": {
                            "nodes": mock_repos
                        }
                    }
                }
            }).to_string())
            .create();

        let result = get_user_github_repos_using_graphql_api(mock_access_token).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), mock_repos);

        mock.assert();
    }

    #[test]
    fn test_get_user_github_repos_using_graphql_api_error() {
        let mock_access_token = "test_token";

        let mock = mock("GET", "/graphql")
            .with_status(500)
            .create();

        let result = get_user_github_repos_using_graphql_api(mock_access_token).await;

        assert!(result.is_err());

        mock.assert();
    }
}
