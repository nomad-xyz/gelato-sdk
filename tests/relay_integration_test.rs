use ethers_core::types::H160;
use gelato_sdk::*;

#[tokio::test]
async fn simple_queries() -> Result<(), reqwest::Error> {
    let gelato = GelatoClient::default();

    // Ensure calling get chains returns non-empty array
    let chains = gelato.get_gelato_relay_chains().await.unwrap();
    assert!(chains.len() > 0);

    // Ensure calling get task status returns a result
    let task_status = gelato
        .get_task_status(
            "0xce52ae7a6a3032848d76b161ac4c131fa995dcc67e3be5392dfb8466275d6679"
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(task_status.is_some());

    // Ensure we calling estimate fee on mainnet ethereum doesn't return error
    let mainnet: usize = chains[0];
    let _ = gelato
        .get_estimated_fee(
            mainnet,
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
                .parse::<H160>()
                .unwrap(),
            100_000,
            true,
        )
        .await
        .unwrap();

    Ok(())
}
