use anyhow::{anyhow, Result};
use apollo_router::subscriber::{set_global_subscriber, RouterSubscriber};
use apollo_router_core::{plugin_utils, PluggableRouterServiceBuilder};
use std::sync::Arc;
use tower::{util::BoxService, ServiceExt};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // set up console logs
    let builder = tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::try_new("info").expect("could not parse log"));

    set_global_subscriber(RouterSubscriber::TextSubscriber(builder.finish()))?;

    // get the supergraph from ../../examples/graphql/supergraph.graphql
    let schema = Arc::new(include_str!("../../graphql/supergraph.graphql").parse()?);

    // PluggableRouterServiceBuilder creates a GraphQL pipeline to process queries against a supergraph Schema
    // The whole pipeline is set up...
    let mut router_builder = PluggableRouterServiceBuilder::new(schema);

    // ... except the SubgraphServices, so we'll let it know Requests against the `accounts` service
    // can be performed with an http client against the `https://accounts.demo.starstuff.dev` url
    let subgraph_service = BoxService::new(apollo_router_core::ReqwestSubgraphService::new(
        "accounts".to_string(),
    ));
    router_builder = router_builder.with_subgraph_service("accounts", subgraph_service);

    // We can now build our service stack...
    let (router_service, _) = router_builder.build().await;

    // ...then create a GraphQL request...
    let request = plugin_utils::RouterRequest::builder()
        .query(r#"query Query { me { name } }"#.to_string())
        .build()
        .into();

    // ... and run it against the router service!
    let res = router_service
        .oneshot(request)
        .await
        .map_err(|e| anyhow!("router_service call failed: {}", e))?;

    // {
    //   "data": {
    //     "me": {
    //       "name": "Ada Lovelace"
    //     }
    //   }
    // }
    println!("{}", serde_json::to_string_pretty(res.response.body())?);
    Ok(())
}
