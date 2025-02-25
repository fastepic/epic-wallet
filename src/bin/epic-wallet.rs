// Copyright 2019 The Epic Developers
//
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

//! Main for building the binary of a Epic Reference Wallet

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;
use crate::core::global;
use crate::util::init_logger;
use clap::App;
use epic_wallet::cmd;
use epic_wallet_config as config;
use epic_wallet_impls::HTTPNodeClient;
use epic_wallet_util::epic_core as core;
use epic_wallet_util::epic_util as util;
use std::env;
use std::fs;
use std::path::PathBuf;

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn info_strings() -> (String, String) {
	(
		format!(
			"This is Epic Wallet version {}{}, built for {} by {}.",
			built_info::PKG_VERSION,
			built_info::GIT_VERSION.map_or_else(|| "".to_owned(), |v| format!(" (git {})", v)),
			built_info::TARGET,
			built_info::RUSTC_VERSION,
		)
		.to_string(),
		format!(
			"Built with profile \"{}\", features \"{}\".",
			built_info::PROFILE,
			built_info::FEATURES_STR,
		)
		.to_string(),
	)
}

fn log_build_info() {
	let (basic_info, detailed_info) = info_strings();
	info!("{}", basic_info);
	debug!("{}", detailed_info);
}

fn main() {
	let exit_code = real_main();
	std::process::exit(exit_code);
}

fn real_main() -> i32 {
	let yml = load_yaml!("epic-wallet.yml");
	let args = App::from_yaml(yml)
		.version(built_info::PKG_VERSION)
		.get_matches();

	let chain_type = if args.is_present("floonet") {
		global::ChainTypes::Floonet
	} else if args.is_present("usernet") {
		global::ChainTypes::UserTesting
	} else {
		global::ChainTypes::Mainnet
	};

	let mut current_dir = None;
	if let Some(_path) = args.value_of("current_dir") {
		let current_dir_exist = PathBuf::from(&_path);
		if !current_dir_exist.exists() {
			fs::create_dir_all(current_dir_exist.clone()).unwrap_or_else(|e| {
				panic!("Error creating current_dir: {:?}", e);
			});
		}
		current_dir = Some(current_dir_exist);
	}
	// special cases for certain lifecycle commands
	match args.subcommand() {
		("init", Some(init_args)) => {
			if init_args.is_present("here") {
				current_dir = Some(env::current_dir().unwrap_or_else(|e| {
					panic!("Error creating config file: {}", e);
				}));
			}
		}
		_ => {}
	}

	// Load relevant config, try and load a wallet config file
	// Use defaults for configuration if config file not found anywhere
	let mut config = config::initial_setup_wallet(&chain_type, current_dir).unwrap_or_else(|e| {
		panic!("Error loading wallet configuration: {}", e);
	});

	//config.members.as_mut().unwrap().wallet.chain_type = Some(chain_type);

	// Load logging config
	let l = config.members.as_mut().unwrap().logging.clone().unwrap();
	init_logger(Some(l), None);
	info!(
		"Using wallet configuration file at {}",
		config.config_file_path.as_ref().unwrap().to_str().unwrap()
	);

	log_build_info();

	global::set_mining_mode(
		config
			.members
			.as_ref()
			.unwrap()
			.wallet
			.chain_type
			.as_ref()
			.unwrap()
			.clone(),
	);

	let wallet_config = config.clone().members.unwrap().wallet;
	let node_client = HTTPNodeClient::new(&wallet_config.check_node_api_http_addr, None);

	cmd::wallet_command(&args, config, node_client)
}
