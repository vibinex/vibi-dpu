use super::*;

#[cfg(test)]
mod tests {
    use std::{fmt::Debug, result};

    use super::*;
    // use futures_util::TryFutureExt;
    // use mockito::mock;

    #[tokio::test]
    async fn test_get_user_github_repos_using_graphql_api_success() {
        let mock_access_token = "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";

        log::info!("Ola amigos, your mock has been created!");
        let result = get_user_github_repos_using_graphql_api(mock_access_token).await;

        // print the values in the result object
        println!("Result: {:?}", result);
        // if (result.is_err()) {
        //     println!("Error: {:?}", result.err());
        // }
        // assert that the result is successful
        assert_eq!(1,2)
        
        // assert!(result.is_ok());
        // assert_eq!(result.unwrap(), mock_repos);

        // mock.assert();
    }

    // #[test]
    // fn test_get_user_github_repos_using_graphql_api_error() {
    //     let mock_access_token = "test_token";

    //     let mock = mock("GET", "/graphql")
    //         .with_status(500)
    //         .create();

    //     let result = get_user_github_repos_using_graphql_api(mock_access_token).await;

    //     assert!(result.is_err());

    //     mock.assert();
    // }

    #[tokio::test]
async fn test_query_github_graphql_api_for_repos() {
  let access_token = "YOUR AUTH TOKEN";

  let client = reqwest::Client::new();
  
  let query = "query { viewer { repositories(first: 100, affiliations: [OWNER, ORGANIZATION_MEMBER, COLLABORATOR], ownerAffiliations: [OWNER, ORGANIZATION_MEMBER, COLLABORATOR]) { totalCount nodes { name id isPrivate sshUrl owner { login } } } } }";
  let body = json!({
    "query": query
  });
  
  println!("Executing GraphQL query: {:?}", body);
  let graphql_request = client
    .post("https://api.github.com/graphql")
    .header("Authorization", "Bearer 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") 
    .header("Content-Type", "application/json")
    .header("User-Agent", "vibi-dpu")
    .json(&body).build().unwrap();
  println!("Request Headers: {:?}", graphql_request.headers());
  println!("Request URL: {:?}", graphql_request.url());
  let response = client.execute(graphql_request)
    .await
    .expect("Failed to execute request");

let status = response.status();
let resp_body = response.text().await.unwrap();

if status.is_success() {
    println!("Response Status: {}", status);
    println!("Response Body: {}", resp_body);
} else {
    println!("Failed response with status: {}", status);
    println!("Response Body: {}", resp_body);
}

assert!(!status.is_success());
  // Assert on response JSON here
}

}
