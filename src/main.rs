mod consts {
    pub const VERSION: &str = "0.1.0";
    pub const REPOSITORY: &str = "https://github.com/vlad2030/solana-balance-monitor";
    pub const AUTHOR: &str = "lalka2003";
    pub const AUTHOR_LINK: &str = "https://t.me/chad_trade";
}

mod config {
    use crate::consts;

    #[derive(Clone, Debug)]
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Config {
        pub version: String,
    }

    impl Config {
        pub fn default() -> Self {
            Self {
                version: consts::VERSION.into(),
            }
        }
        pub async fn get(&self) -> Config {
            let client: reqwest::Client = reqwest::Client::new();
            let mut headers: reqwest::header::HeaderMap = reqwest::header::HeaderMap::new();
            headers.insert(
                "User-Agent",
                reqwest::header::HeaderValue::from_str("solana-balance-monitor").unwrap(),
            );
            headers.insert(
                "X-GitHub-Api-Version",
                reqwest::header::HeaderValue::from_str("2022-11-28").unwrap(),
            );
            headers.insert(
                "Accept",
                reqwest::header::HeaderValue::from_str("application/vnd.github.raw+json").unwrap(),
            );

            let config: Config = client
                .request(reqwest::Method::GET, consts::REPOSITORY)
                .headers(headers)
                .send()
                .await
                .unwrap()
                .json::<Config>()
                .await
                .unwrap_or(Config::default());

            config
        }
    }
}

mod wallet {
    use crate::utils;

    #[derive(Clone, Debug)]
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Wallet {
        pub address: String,
        pub lamports: Option<u64>,
        pub initial_lamports: Option<u64>,
        pub price: Option<u64>,
        pub updated: u64,
    }

    impl Wallet {
        pub fn default() -> Self {
            let args: Vec<String> = std::env::args().collect();
            let address = if args.len() > 1 {
                &args[1]
            } else {
                ""
            };

            Self {
                address: address.to_owned(),
                lamports: None,
                initial_lamports: None,
                price: None,
                updated: utils::get_epoch_time(),
            }
        }
    }
}

mod solana {
    pub const MAINNET_URL: &str = "https://api.mainnet-beta.solana.com";
    pub const DEVNET_URL: &str = "https://api.devnet.solana.com";
    pub const TESTNET_URL: &str = "https://api.testnet.solana.com";

    pub struct Solana {
        client: solana_client::nonblocking::rpc_client::RpcClient,
    }

    impl Solana {
        pub fn new(node_url: &str) -> Self {
            let client: solana_client::nonblocking::rpc_client::RpcClient =
                solana_client::nonblocking::rpc_client::RpcClient::new(node_url.to_string());

            Self {
                client,
            }
        }

        pub async fn get_balance(
            &self,
            pubkey: &solana_program::pubkey::Pubkey,
        ) -> solana_client::client_error::Result<u64> {
            self.client.get_balance(pubkey).await
        }
    }
}

mod jupiter {}

mod utils {
    use std::str::FromStr;

    pub async fn check_internet_connection() -> bool {
        let response: Result<reqwest::Response, reqwest::Error> = reqwest::Client::new()
            .get("https://cloudflare.com/cdn-cgi/trace")
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;

        response.is_ok()
    }

    pub fn is_sol_address(address: &str) -> bool {
        let re = regex::Regex::new(r"[1-9A-HJ-NP-Za-km-z]{32,44}").unwrap();

        re.is_match(address)
    }

    pub fn shortify_sol_address(address: &str) -> String {
        format!(
            "{}..{}",
            &address[..4],
            &address[address.len() - 4..]
        )
    }

    pub fn str_sol_address_to_pubkey(address: &str) -> solana_program::pubkey::Pubkey {
        solana_program::pubkey::Pubkey::from_str(address)
            .unwrap_or(solana_program::pubkey::Pubkey::default())
    }

    pub fn lamports_to_sol(lamports: u64) -> f64 {
        solana_sdk::native_token::lamports_to_sol(lamports)
    }

    pub fn get_epoch_time() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    pub fn epoch_to_datetime(time: u64) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(time as i64, 0).unwrap_or_default()
    }
}

mod app {
    use crate::config;
    use crate::solana;
    use crate::utils;
    use crate::wallet;

    #[derive(Clone, Debug)]
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct App {
        pub config: config::Config,
        pub wallet: wallet::Wallet,
    }

    impl App {
        pub async fn default() -> Self {
            let app_config: config::Config = config::Config::default().get().await;
            let app_wallet: wallet::Wallet = wallet::Wallet::default();

            Self {
                config: app_config,
                wallet: app_wallet,
            }
        }

        pub async fn run(
            &mut self,
            mut terminal: ratatui::DefaultTerminal,
        ) -> Result<(), ()> {
            let solana_client: solana::Solana = solana::Solana::new(solana::MAINNET_URL);
            let address: String = self.wallet.clone().address;
            let pubkey = utils::str_sol_address_to_pubkey(address.as_str());
            let initial_lamports = self.wallet.clone().initial_lamports;

            if initial_lamports.is_none() {
                let address_balance: u64 = solana_client
                    .get_balance(&pubkey)
                    .await
                    .unwrap_or(0);

                self.wallet.initial_lamports = Some(address_balance);
                self.wallet.lamports = Some(address_balance);
            }

            loop {
                let address_balance: u64 = solana_client
                    .get_balance(&pubkey)
                    .await
                    .unwrap_or(1);

                self.wallet.lamports = Some(address_balance);
                self.wallet.updated = utils::get_epoch_time();

                terminal.draw(|frame| self.draw(frame)).unwrap();

                // if let ratatui::crossterm::event::Event::Key(key) =
                //     event
                // {
                //     match key.code {
                //         ratatui::crossterm::event::KeyCode::Char('q') => break Ok(()),
                //         _ => { continue; },
                //     }

                if ratatui::crossterm::event::poll(std::time::Duration::from_millis(100)).unwrap()
                {
                    if let ratatui::crossterm::event::Event::Key(key) =
                        ratatui::crossterm::event::read().unwrap()
                    {
                        if key.kind == ratatui::crossterm::event::KeyEventKind::Press
                            && key.code == ratatui::crossterm::event::KeyCode::Char('q')
                        {
                            break Ok(());
                        }
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await
            }
        }

        pub fn draw(
            &mut self,
            frame: &mut ratatui::Frame,
        ) {
            let layout = self.calculate_layout(frame.area());
            let main_layout = layout.clone();

            self.render(frame, main_layout);
        }

        pub fn render(
            &mut self,
            frame: &mut ratatui::Frame,
            layout: Vec<Vec<ratatui::layout::Rect>>,
        ) {
            frame.render_widget(
                ratatui::widgets::Block::new()
                    .title_top("Solana balance monitor (q to exit)")
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .borders(ratatui::widgets::Borders::TOP),
                layout[0][0],
            );

            let short_address: String = utils::shortify_sol_address(&self.wallet.address);
            let sol_balance = utils::lamports_to_sol(self.wallet.lamports.unwrap_or(0));
            let sol_initial_balance =
                utils::lamports_to_sol(self.wallet.initial_lamports.unwrap_or(0));
            let datetime: String = utils::epoch_to_datetime(self.wallet.updated)
                .format("%H:%M:%S %p")
                .to_string();

            frame.render_widget(
                ratatui::widgets::Paragraph::new(
                    vec![
                        ratatui::prelude::Line::from(ratatui::text::Span::raw(format!(
                            "Wallet: {}",
                            short_address
                        ))),
                        ratatui::prelude::Line::from(ratatui::text::Span::raw(format!(
                            "Starting balance: {:.4} SOL",
                            sol_initial_balance
                        ))),
                        ratatui::prelude::Line::from(ratatui::text::Span::raw(format!(
                            "Current balance: {:.4} SOL",
                            sol_balance
                        ))),
                        ratatui::prelude::Line::from(ratatui::text::Span::raw(format!(
                            "Last updated: {}",
                            datetime
                        ))),
                    ], // ratatui::prelude::Line::from(vec![
                       // ratatui::text::Span::raw(format!("Wallet: {}\n", short_address)),
                       // ratatui::text::Span::raw(format!(
                       //     "Starting balance: {} SOL\n",
                       //     sol_initial_balance
                       // )),
                       // ratatui::text::Span::raw(format!("Current balance: {} SOL\n", sol_balance)),
                       // ratatui::text::Span::raw(format!("Last updated: {}\n", datetime)),
                )
                .alignment(ratatui::layout::Alignment::Center)
                .block(ratatui::widgets::Block::new()),
                layout[0][1],
            );
        }

        pub fn calculate_layout(
            &self,
            area: ratatui::layout::Rect,
        ) -> Vec<Vec<ratatui::layout::Rect>> {
            let main_layout = ratatui::layout::Layout::vertical([
                ratatui::layout::Constraint::Percentage(10),
                ratatui::layout::Constraint::Percentage(90),
                ratatui::layout::Constraint::Percentage(10),
            ]);
            let block_layout = ratatui::layout::Layout::vertical(
                [ratatui::layout::Constraint::Percentage(100); 1],
            );
            let [_, main_area, _] = main_layout.areas(area);
            let main_areas = block_layout
                .split(main_area)
                .iter()
                .map(|&area| {
                    ratatui::layout::Layout::vertical([
                        ratatui::layout::Constraint::Percentage(50),
                        ratatui::layout::Constraint::Percentage(50),
                    ])
                    .split(area)
                    .to_vec()
                })
                .collect::<Vec<Vec<ratatui::layout::Rect>>>();

            main_areas
        }

        pub fn init(&self) -> ratatui::DefaultTerminal {
            ratatui::init()
        }

        pub fn restore(&self) {
            ratatui::restore();
        }
    }
}

#[tokio::main]
async fn main() {
    println!(
        "Created by {} ({})\n",
        consts::AUTHOR,
        consts::AUTHOR_LINK
    );

    println!("Checking internet connection..");
    let internet_connection: bool = utils::check_internet_connection().await;

    if !internet_connection {
        println!("No internet connection!");
        return;
    }

    println!("Checking for updates..");
    let application: app::App = app::App::default().await;

    if application.config.version.as_str() != consts::VERSION {
        println!(
            "New update is available ({} > {})!",
            consts::VERSION,
            application.config.version
        );
        print!("Get it on {}\n\n", consts::REPOSITORY);
    };

    if !utils::is_sol_address(application.wallet.address.as_str()) {
        println!("Your solana address is not valid!");
        return;
    }

    println!("Everything is OK!");
    let terminal = application.init();
    let _ = application.clone().run(terminal).await;

    application.restore()
}
