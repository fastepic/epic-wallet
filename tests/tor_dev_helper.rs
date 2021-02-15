// Copyright 2019 The Epic Developers
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;

extern crate epic_wallet;

use clap::App;
use epic_wallet_impls::test_framework::{self, LocalWalletClient, WalletProxy};
use epic_wallet_util::epic_core::core::feijoada;
use epic_wallet_util::epic_core::global::{self, ChainTypes};
use std::thread;
use std::time::Duration;

use epic_wallet_impls::DefaultLCProvider;
use epic_wallet_util::epic_keychain::ExtKeychain;

use epic_wallet_util::epic_util as util;

#[macro_use]
mod common;
use common::{execute_command, initial_setup_wallet, instantiate_wallet};

// Development testing helper for tor/socks investigation.
// Not (yet) to be run as part of automated testing

fn setup_no_clean() {
	util::init_test_logger();
	global::set_mining_mode(ChainTypes::AutomatedTesting);
	global::set_foundation_path("../tests/assets/foundation.json".to_string());
	let mut policies: feijoada::Policy = feijoada::get_bottles_default();
	policies.insert(feijoada::PoWType::Cuckatoo, 100);
	global::set_policy_config(feijoada::PolicyConfig {
		policies: vec![policies.clone()],
		..Default::default()
	});
}

#[ignore]
#[test]
fn socks_tor() -> Result<(), epic_wallet_controller::Error> {
	let test_dir = "target/test_output/socks_tor";
	let yml = load_yaml!("../src/bin/epic-wallet.yml");
	let app = App::from_yaml(yml);
	setup_no_clean();

	setup_proxy!(test_dir, chain, wallet1, client1, mask1, wallet2, client2, _mask2);

	// Tor should be running at this point for wallet 2, with a hidden service
	// bound to the listening port 53415. By default, tor will also be running
	// a socks proxy lister at 127.0.0.1 9050 (both wallets can use for now)
	//
	// Relevant torrc config:
	// HiddenServiceDir ./hidden_service/
	// HiddenServicePort 80 127.0.0.1:53415
	//
	// tor -f torrc

	// Substitute whatever onion address has been created
	let onion_address = "2a6at2obto3uvkpkitqp4wxcg6u36qf534eucbskqciturczzc5suyid";

	// run the foreign listener for wallet 2
	let arg_vec = vec!["epic-wallet", "-p", "password", "listen"];
	// Set owner listener running
	thread::spawn(move || {
		let yml = load_yaml!("../src/bin/epic-wallet.yml");
		let app = App::from_yaml(yml);
		execute_command(&app, test_dir, "wallet2", &client2, arg_vec.clone()).unwrap();
	});

	// dumb pause for now, hidden service should already be running
	thread::sleep(Duration::from_millis(3000));

	// mine into wallet 1 a bit
	let bh = 5u64;
	let _ =
		test_framework::award_blocks_to_wallet(&chain, wallet1.clone(), mask1, bh as usize, false);

	// now, test send from wallet 1 over tor
	let arg_vec = vec![
		"epic-wallet",
		"-p",
		"password",
		"send",
		"-c",
		"2",
		"-d",
		onion_address,
		"10",
	];
	execute_command(&app, test_dir, "wallet1", &client1, arg_vec)?;

	Ok(())
}
