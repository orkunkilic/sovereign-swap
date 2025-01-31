#[cfg(test)]
pub mod test {
    use crate::{
        genesis_config::{DEMO_SEQUENCER_DA_ADDRESS, LOCKED_AMOUNT},
        runtime::Runtime,
        tests::{
            create_demo_config, create_new_demo, data_generation::simulate_da, has_tx_events,
            new_test_blob, C,
        },
    };
    use sov_modules_api::{
        default_context::DefaultContext, default_signature::private_key::DefaultPrivateKey,
    };
    use sov_modules_stf_template::{Batch, SequencerOutcome};
    use sov_rollup_interface::{mocks::MockZkvm, stf::StateTransitionFunction};
    use sov_state::{ProverStorage, WorkingSet};

    #[test]
    fn test_demo_values_in_db() {
        let path = sov_schema_db::temppath::TempPath::new();
        let value_setter_admin_private_key = DefaultPrivateKey::generate();
        let election_admin_private_key = DefaultPrivateKey::generate();

        let config = create_demo_config(
            LOCKED_AMOUNT + 1,
            &value_setter_admin_private_key,
            &election_admin_private_key,
        );
        {
            let mut demo = create_new_demo(&path);

            StateTransitionFunction::<MockZkvm>::init_chain(&mut demo, config);
            StateTransitionFunction::<MockZkvm>::begin_slot(&mut demo, Default::default());

            let txs = simulate_da(value_setter_admin_private_key, election_admin_private_key);

            let apply_blob_outcome = StateTransitionFunction::<MockZkvm>::apply_blob(
                &mut demo,
                new_test_blob(Batch { txs }, &DEMO_SEQUENCER_DA_ADDRESS),
                None,
            );

            assert!(
                matches!(apply_blob_outcome.inner, SequencerOutcome::Rewarded(0),),
                "Sequencer execution should have succeeded but failed "
            );

            assert!(has_tx_events(&apply_blob_outcome),);

            StateTransitionFunction::<MockZkvm>::end_slot(&mut demo);
        }

        // Generate a new storage instance after dumping data to the db.
        {
            let runtime = &mut Runtime::<DefaultContext>::new();
            let storage = ProverStorage::with_path(&path).unwrap();
            let mut working_set = WorkingSet::new(storage);

            let resp = runtime.election.results(&mut working_set);

            assert_eq!(
                resp,
                sov_election::query::GetResultResponse::Result(Some(sov_election::Candidate {
                    name: "candidate_2".to_owned(),
                    count: 3
                }))
            );
            let resp = runtime.value_setter.query_value(&mut working_set);

            assert_eq!(resp, sov_value_setter::query::Response { value: Some(33) });
        }
    }

    #[test]
    fn test_demo_values_in_cache() {
        let path = sov_schema_db::temppath::TempPath::new();
        let mut demo = create_new_demo(&path);

        let value_setter_admin_private_key = DefaultPrivateKey::generate();
        let election_admin_private_key = DefaultPrivateKey::generate();

        let config = create_demo_config(
            LOCKED_AMOUNT + 1,
            &value_setter_admin_private_key,
            &election_admin_private_key,
        );

        StateTransitionFunction::<MockZkvm>::init_chain(&mut demo, config);
        StateTransitionFunction::<MockZkvm>::begin_slot(&mut demo, Default::default());

        let txs = simulate_da(value_setter_admin_private_key, election_admin_private_key);

        let apply_blob_outcome = StateTransitionFunction::<MockZkvm>::apply_blob(
            &mut demo,
            new_test_blob(Batch { txs }, &DEMO_SEQUENCER_DA_ADDRESS),
            None,
        );

        assert!(
            matches!(apply_blob_outcome.inner, SequencerOutcome::Rewarded(0),),
            "Sequencer execution should have succeeded but failed "
        );

        assert!(has_tx_events(&apply_blob_outcome),);

        StateTransitionFunction::<MockZkvm>::end_slot(&mut demo);

        let runtime = &mut Runtime::<DefaultContext>::new();
        let mut working_set = WorkingSet::new(demo.current_storage.clone());

        let resp = runtime.election.results(&mut working_set);

        assert_eq!(
            resp,
            sov_election::query::GetResultResponse::Result(Some(sov_election::Candidate {
                name: "candidate_2".to_owned(),
                count: 3
            }))
        );

        let resp = runtime.value_setter.query_value(&mut working_set);

        assert_eq!(resp, sov_value_setter::query::Response { value: Some(33) });
    }

    #[test]
    fn test_demo_values_not_in_db() {
        let path = sov_schema_db::temppath::TempPath::new();

        let value_setter_admin_private_key = DefaultPrivateKey::generate();
        let election_admin_private_key = DefaultPrivateKey::generate();

        let config = create_demo_config(
            LOCKED_AMOUNT + 1,
            &value_setter_admin_private_key,
            &election_admin_private_key,
        );
        {
            let mut demo = create_new_demo(&path);

            StateTransitionFunction::<MockZkvm>::init_chain(&mut demo, config);
            StateTransitionFunction::<MockZkvm>::begin_slot(&mut demo, Default::default());

            let txs = simulate_da(value_setter_admin_private_key, election_admin_private_key);

            let apply_blob_outcome = StateTransitionFunction::<MockZkvm>::apply_blob(
                &mut demo,
                new_test_blob(Batch { txs }, &DEMO_SEQUENCER_DA_ADDRESS),
                None,
            )
            .inner;
            assert!(
                matches!(apply_blob_outcome, SequencerOutcome::Rewarded(0),),
                "Sequencer execution should have succeeded but failed "
            );
        }

        // Generate a new storage instance, value are missing because we didn't call `end_slot()`;
        {
            let runtime = &mut Runtime::<C>::new();
            let storage = ProverStorage::with_path(&path).unwrap();
            let mut working_set = WorkingSet::new(storage);

            let resp = runtime.election.results(&mut working_set);

            assert_eq!(
                resp,
                sov_election::query::GetResultResponse::Err("Election is not frozen".to_owned())
            );

            let resp = runtime.value_setter.query_value(&mut working_set);

            assert_eq!(resp, sov_value_setter::query::Response { value: None });
        }
    }

    #[test]
    fn test_sequencer_insufficient_funds() {
        let path = sov_schema_db::temppath::TempPath::new();

        let value_setter_admin_private_key = DefaultPrivateKey::generate();
        let election_admin_private_key = DefaultPrivateKey::generate();

        let config = create_demo_config(
            LOCKED_AMOUNT - 1,
            &value_setter_admin_private_key,
            &election_admin_private_key,
        );

        let mut demo = create_new_demo(&path);

        StateTransitionFunction::<MockZkvm>::init_chain(&mut demo, config);
        StateTransitionFunction::<MockZkvm>::begin_slot(&mut demo, Default::default());

        let txs = simulate_da(value_setter_admin_private_key, election_admin_private_key);

        let apply_blob_outcome = StateTransitionFunction::<MockZkvm>::apply_blob(
            &mut demo,
            new_test_blob(Batch { txs }, &DEMO_SEQUENCER_DA_ADDRESS),
            None,
        );

        assert!(
            matches!(apply_blob_outcome.inner, SequencerOutcome::Ignored),
            "Batch should have been skipped due to insufficient funds"
        );

        // Assert that there are no events
        assert!(!has_tx_events(&apply_blob_outcome));
    }
}
