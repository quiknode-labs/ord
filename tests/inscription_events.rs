use super::*;

#[test]
fn index_inscription_events_without_addresses_fails() {
  let core = mockcore::spawn();

  CommandBuilder::new("--index-inscription-events server")
    .core(&core)
    .expected_exit_code(1)
    .stderr_regex(".*--index-inscription-events requires --index-addresses.*")
    .run_and_extract_stdout();
}

#[test]
fn r_events_endpoints_return_inscription_history() {
  let core = mockcore::spawn();
  let ord = TestServer::spawn_with_server_args(
    &core,
    &["--index-addresses", "--index-inscription-events"],
    &[],
  );

  create_wallet(&core, &ord);

  let (inscription_id, _reveal) = inscribe(&core, &ord);

  let response = ord.json_request(format!("/r/events/inscription/{inscription_id}/0"));
  assert_eq!(response.status(), StatusCode::OK);

  let events: api::InscriptionEvents = serde_json::from_str(&response.text().unwrap()).unwrap();

  assert_eq!(events.page, 0);
  assert!(!events.more);
  assert_eq!(events.events.len(), 1);
  assert_eq!(events.events[0].event_type, "created");
  assert_eq!(events.events[0].inscription_id, inscription_id);
  assert!(events.events[0].new_satpoint.is_some());
  assert!(events.events[0].old_satpoint.is_none());
  assert!(events.events[0].to_address.is_some());
  assert!(events.events[0].from_address.is_none());

  let destination = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";

  CommandBuilder::new(format!(
    "wallet send --fee-rate 1 {destination} {inscription_id}"
  ))
  .core(&core)
  .ord(&ord)
  .run_and_deserialize_output::<Send>();

  core.mine_blocks(1);

  let response = ord.json_request(format!("/r/events/inscription/{inscription_id}/0"));
  let events: api::InscriptionEvents = serde_json::from_str(&response.text().unwrap()).unwrap();

  assert_eq!(events.events.len(), 2);
  assert_eq!(events.events[0].event_type, "created");
  assert_eq!(events.events[1].event_type, "transferred");
  assert_eq!(events.events[1].to_address.as_deref(), Some(destination));
  assert!(events.events[1].from_address.is_some());
  assert!(events.events[1].old_satpoint.is_some());
  assert!(events.events[1].new_satpoint.is_some());

  let transfer_block = events.events[1].block_height;
  let response = ord.json_request(format!(
    "/r/events/block/{transfer_block}/{transfer_block}/0"
  ));
  let block_events: api::InscriptionEvents =
    serde_json::from_str(&response.text().unwrap()).unwrap();

  assert!(
    block_events
      .events
      .iter()
      .any(|e| e.inscription_id == inscription_id && e.event_type == "transferred")
  );
}
