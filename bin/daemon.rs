use clap::{Args, Parser, Subcommand};
use daemonize::Daemonize;
use futures::lock::Mutex;
use libc::kill;
use rings_node::{
    logger::{LogLevel, Logger},
    prelude::rings_core::{
        async_trait,
        dht::{Did, PeerRing, Stabilization, TStabilize},
        ecc::SecretKey,
        message::{self, CustomMessage, Message, MessageHandler, MessageRelay},
        prelude::url,
        session::SessionManager,
        swarm::Swarm,
        types::message::MessageListener,
    },
    service::{run_service, run_stabilize, run_udp_turn},
};
use std::{
    fs::{self, File},
    str::FromStr,
    sync::Arc,
};
use tokio::signal;

#[derive(Parser, Debug)]
#[clap(about)]
struct Cli {
    #[clap(long, short = 'v', default_value_t = LogLevel::Info, arg_enum)]
    log_level: LogLevel,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Run(Box<RunArgs>),
    Shutdown(ShutdownArgs),
}

#[derive(Args, Debug)]
struct RunArgs {
    #[clap(long, short = 'b', default_value = "127.0.0.1:50000", env)]
    pub http_addr: String,

    #[clap(long, short = 's', default_value = "stun://stun.l.google.com:19302")]
    pub ice_server: Vec<String>,

    #[clap(
        long = "eth",
        short = 'e',
        default_value = "http://127.0.0.1:8545",
        env
    )]
    pub eth_endpoint: String,

    #[clap(long = "key", short = 'k', env)]
    pub eth_key: SecretKey,

    #[clap(short = 'd')]
    pub daemonize: bool,

    #[clap(long, short = 'p', default_value = "/tmp/rings-node.pid")]
    pub pid_file: String,

    #[clap(long, default_value = "nobody")]
    pub user: String,

    #[clap(long, default_value = "daemon")]
    pub group: String,

    #[clap(long, short = 'w', default_value = "/")]
    pub work_dir: String,

    /// STUN server address.
    #[clap(long, default_value = "3478")]
    pub turn_port: u16,

    /// STUN publicip.
    #[clap(long, default_value = "127.0.0.1")]
    pub public_ip: String,

    /// Username.
    #[clap(long, default_value = "rings")]
    pub turn_username: String,

    /// Password.
    #[clap(long, default_value = "password")]
    pub turn_password: String,

    /// Realm.
    /// REALM
    /// The REALM attribute is present in Shared Secret Requests and Shared
    /// Secret Responses. It contains text which meets the grammar for
    /// "realm" as described in RFC 3261, and will thus contain a quoted
    /// string (including the quotes).
    #[clap(long, default_value = "rings")]
    pub turn_realm: String,

    #[clap(long)]
    pub without_turn: bool,

    #[clap(long, defualt_value = "20")]
    pub stabilize_timeout: usize,
}

#[derive(Args, Debug)]
struct ShutdownArgs {
    #[clap(long, short = 'p', default_value = "/tmp/rings-node.pid")]
    pub pid_file: String,
}

async fn run_jobs(args: &RunArgs) -> anyhow::Result<()> {
    let key: &SecretKey = &args.eth_key;
    let dht = Arc::new(Mutex::new(PeerRing::new(key.address().into())));

    let (auth, s_key) = SessionManager::gen_unsign_info(
        key.address(),
        Some(rings_core::session::Ttl::Never),
        None,
    )?;
    let sig = key.sign(&auth.to_string()?).to_vec();
    let session = SessionManager::new(&sig, &auth, &s_key);

    let mut ice_servers = args.ice_server.clone();
    let turn_server = if !args.without_turn {
        let mut turn_url = url::Url::from_str("turn://0.0.0.0:3567").unwrap();
        turn_url.set_port(Some(args.turn_port)).unwrap();
        turn_url.set_username(args.turn_username.as_str()).unwrap();
        turn_url
            .set_password(Some(args.turn_password.as_str()))
            .unwrap();
        ice_servers.push(turn_url.to_string());
        Some(
            run_udp_turn(
                args.public_ip.as_str(),
                args.turn_port,
                args.turn_username.as_str(),
                args.turn_password.as_str(),
                args.turn_realm.as_str(),
            )
            .await?,
        )
    } else {
        None
    };

    let ice_servers = ice_servers.join(";");
    let swarm = Arc::new(Swarm::new(&ice_servers, key.address(), session));

    // let listen_event = MessageHandler::new(dht.clone(), swarm.clone());
    let message_callback = MessageCallback {};
    let listen_event = Arc::new(MessageHandler::new_with_callback(
        dht.clone(),
        swarm.clone(),
        Box::new(message_callback),
    ));
    let stabilization = Arc::new(Stabilization::new(
        dht.clone(),
        swarm.clone(),
        args.stabilize_timeout,
    ));
    let http_addr = args.http_addr.clone();
    let j = tokio::spawn(futures::future::join(
        async {
            listen_event.clone().listen().await;
            AnyhowResult::Ok(())
        },
        async {
            run_service(http_addr, swarm, listen_event).await?;
            AnyhowResult::Ok(())
        },
        async {
            stabilization.wait().await?;
            AnyhowResult::Ok(())
        },
    ));
    signal::ctrl_c().await.expect("failed to listen for event");
    println!("\nClosing connection now...");
    j.abort();
    if let Some(s) = turn_server {
        if let Err(e) = s.close().await {
            println!("close turn_server failed, {}", e);
        }
    }
    println!("Server closed");

    Ok(())
}

type AnyhowResult<T> = Result<T, anyhow::Error>;

struct MessageCallback {}

#[async_trait]
impl message::MessageCallback for MessageCallback {
    async fn custom_message(&self, _relay: MessageRelay<Message>, _prev: Did, msg: CustomMessage) {
        log::info!("[MESSAGE] custom_message: {:?}", msg);
    }
    async fn builtin_message(&self, _relay: MessageRelay<Message>, _prev: Did) {}
}

fn run_daemon(args: &RunArgs) -> AnyhowResult<()> {
    if args.daemonize {
        fs::create_dir_all("/tmp/rings-node")?;
        let stdout = File::create("/tmp/rings-node/info.log")?;
        let stderr = File::create("/tmp/rings-node/err.log")?;

        let daemonize = Daemonize::new()
            .pid_file(args.pid_file.as_str())
            .chown_pid_file(true)
            .working_directory(args.work_dir.as_str())
            .user(args.user.as_str())
            .group(args.group.as_str())
            .stdout(stdout)
            .stderr(stderr);
        if let Err(e) = daemonize.start() {
            panic!("{}", e);
        }
    }
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if let Err(e) = run_jobs(args).await {
            panic!("{}", e);
        }
    });
    Ok(())
}

fn shutdown_daemon(args: &ShutdownArgs) -> anyhow::Result<()> {
    let pid: i32 = fs::read_to_string(args.pid_file.as_str())?.parse()?;
    unsafe {
        kill(pid, 9);
    }
    println!("Killed: {}", pid);
    Ok(())
}

fn main() {
    dotenv::dotenv().ok();
    let cli = Cli::parse();
    Logger::init(cli.log_level.into()).expect("log err");

    match cli.command {
        Command::Run(args) => {
            if let Err(e) = run_daemon(&args) {
                panic!("{}", e);
            }
        }
        Command::Shutdown(args) => {
            if let Err(e) = shutdown_daemon(&args) {
                panic!("{}", e);
            }
        }
    };
}
