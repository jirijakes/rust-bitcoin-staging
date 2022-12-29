#![cfg(feature = "serde")]
#![cfg(feature = "bitcoinconsensus")]
#![cfg(test)]

use std::fs::File;

use bitcoin::consensus::{deserialize, serialize};
use bitcoin::*;
use bitcoin_hashes::hex::FromHex;
use bitcoinconsensus::*;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(crate = "actual_serde")]
struct Test {
    tx: String,
    prevouts: Vec<String>,
    success: Option<ScriptSig>,
    failure: Option<ScriptSig>,
    #[serde(default, rename = "final")]
    is_final: bool,
    index: usize,
    flags: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "actual_serde")]
#[serde(rename_all = "camelCase")]
struct ScriptSig {
    script_sig: String,
    witness: Vec<String>,
}

#[test]
fn script_assets_test() {
    let f = File::open("tests/data/script_assets_test.json").unwrap();
    let tests: Vec<Test> = serde_json::from_reader(&f).unwrap();

    tests.iter().for_each(do_test);
}

fn test_script(
    script_sig: &ScriptSig,
    tx: &mut Transaction,
    prevout: &TxOut,
    index: usize,
    flags: u32,
) -> Result<(), script::Error> {
    if let Some(txin) = tx.input.get_mut(index) {
        let witness: Vec<Vec<u8>> =
            script_sig.witness.iter().map(|s| Vec::from_hex(&s).unwrap()).collect();

        txin.witness = Witness::from_slice(&witness);
        txin.script_sig = ScriptBuf::from_hex(&script_sig.script_sig).unwrap();
    }

    prevout.script_pubkey.verify_with_flags(
        index,
        Amount::from_sat(prevout.value),
        &serialize(tx),
        flags,
    )
}

fn do_test(test: &Test) {
    let mut tx: Transaction = deserialize(&Vec::from_hex(&test.tx).unwrap()).unwrap();
    let prevouts: Vec<TxOut> =
        test.prevouts.iter().map(|v| deserialize(&Vec::from_hex(&v).unwrap()).unwrap()).collect();

    // Skip taproot, no support with current bitcoinconsensus.
    if prevouts.iter().any(|p| p.script_pubkey.is_v1_p2tr()) {
        return;
    }

    assert_eq!(
        prevouts.len(),
        tx.input.len(),
        "Number of prevouts and number of txins of the transaction are not the same."
    );

    let test_flags: u32 = test.flags.split(",").map(string_to_flag).fold(VERIFY_NONE, |a, b| a | b);

    if let Some(success) = &test.success {
        all_flags()
            .into_iter()
            .filter(|&flags| test.is_final || (flags & flags == flags))
            .for_each(|flags| {
                let res = test_script(&success, &mut tx, &prevouts[test.index], test.index, flags);
                assert!(res.is_ok(), "Success: {:?} {:#?}", res, test);
            });
    }

    if let Some(failure) = &test.failure {
        all_flags().into_iter().filter(|&flags| flags & test_flags == test_flags).for_each(
            |flags| {
                let res = test_script(&failure, &mut tx, &prevouts[test.index], test.index, flags);
                assert!(res.is_err(), "Failure: {:?} {:#?}", res, test);
            },
        );
    }
}

fn string_to_flag(s: &str) -> u32 {
    match s {
        "P2SH" => VERIFY_P2SH,
        "DERSIG" => VERIFY_DERSIG,
        "CHECKLOCKTIMEVERIFY" => VERIFY_CHECKLOCKTIMEVERIFY,
        "CHECKSEQUENCEVERIFY" => VERIFY_CHECKSEQUENCEVERIFY,
        "WITNESS" => VERIFY_WITNESS,
        "NULLDUMMY" => VERIFY_NULLDUMMY,
        // "TAPROOT" => VERIFY_TAPROOT,
        _ => VERIFY_NONE,
    }
}

#[rustfmt::skip]
fn all_flags() -> Vec<u32> {
    (0..64)
    // (0..128)
        .filter_map(|i| {
            let mut flag = 0;
            if i &  1 > 0 { flag |= VERIFY_P2SH };
	    if i &  2 > 0 { flag |= VERIFY_DERSIG };
	    if i &  4 > 0 { flag |= VERIFY_NULLDUMMY };
	    if i &  8 > 0 { flag |= VERIFY_CHECKLOCKTIMEVERIFY };
	    if i & 16 > 0 { flag |= VERIFY_CHECKSEQUENCEVERIFY };
	    if i & 32 > 0 { flag |= VERIFY_WITNESS };
	    // if i & 64 > 0 { flag |= VERIFY_TAPROOT };

	    if flag & VERIFY_WITNESS > 0 && flag & VERIFY_P2SH == 0 { None }
	    // else if flag & VERIFY_TAPROOT > 0 && flag & VERIFY_WITNESS == 0 { None }
	    else { Some(flag) }
        })
        .collect()
}
