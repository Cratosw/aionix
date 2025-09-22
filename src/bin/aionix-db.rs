// Aionix 数据库管理 CLI 工具

use aionix::config::AppConfig;
use aionix::db::cli::{parse_args, print_help, CliExecutor};
use std::env;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    // 检查是否请求帮助
    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        return;
    }

    // 加载配置
    let config = match AppConfig::load() {
        Ok(config) => config,
        Err(e) => {
            error!("加载配置失败: {}", e);
            std::process::exit(1);
        }
    };

    // 创建 CLI 执行器
    let executor = match CliExecutor::new(config).await {
        Ok(executor) => executor,
        Err(e) => {
            error!("初始化 CLI 执行器失败: {}", e);
            std::process::exit(1);
        }
    };

    // 解析命令
    let command = match parse_args(args) {
        Ok(command) => command,
        Err(e) => {
            error!("解析命令失败: {}", e);
            print_help();
            std::process::exit(1);
        }
    };

    // 执行命令
    if let Err(e) = executor.execute(command).await {
        error!("执行命令失败: {}", e);
        std::process::exit(1);
    }

    info!("命令执行完成");
}