mod kernel;
mod simulation;
mod tasks;
mod utils;

use clap::Parser;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::kernel::scheduler::Scheduler;
use crate::tasks::alarm::EmergencyAlarmTask;
use crate::tasks::sensor::{HeartbeatData, HeartbeatSensorTask};
use crate::kernel::scheduler::SchedulerMessage;

/// Sistema de Monitoramento Cardíaco em Tempo Real
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Duração da simulação em segundos
    #[arg(short, long, default_value_t = 60)]
    duration: u64,

    /// Arquivo de saída para logs de ECG
    #[arg(short, long, default_value = "cardiac_log.csv")]
    log_file: String,

    /// Média de BPM
    #[arg(short, long, default_value_t = 70)]
    mean_bpm: u32,

    /// Variação de BPM
    #[arg(short, long, default_value_t = 5)]
    bpm_variation: u32,

    /// Probabilidade de interrupções (0-100)
    #[arg(short, long, default_value_t = 5)]
    interrupt_probability: u32,

    /// CSV de entrada com amostras de ECG
    #[arg(long, default_value = "ecg_input.csv")]
    input_csv: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configurar logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    // Analisar argumentos
    let args = Args::parse();

    // Criar escalonador
    let scheduler = Scheduler::new();

    // Criar estado compartilhado
    let heart_data = Arc::new(Mutex::new(HeartbeatData {
        mv: args.mean_bpm as f32,
        timestamp: chrono::Local::now(),
        is_anomaly: false,
    }));

    // Criar tarefa de alarme para interrupções vindas do sensor
    let alarm_task = EmergencyAlarmTask::new(2, Arc::clone(&heart_data));
    scheduler.add_task(alarm_task.get_task_definition()).await?;

    // Cabeçalho do arquivo de log (output)
    std::fs::write(&args.log_file, "timestamp,ecg_mv,status\n")?;

    // Spawn sensor.run em background para ler e enviar interrupções
    let sensor_tx = scheduler.tx.clone();
    let interrupt_prob = args.interrupt_probability as f64 / 100.0;
    let log_file = args.log_file.clone();
    let run_handle = tokio::spawn(async move {
        let mut sensor_task = HeartbeatSensorTask::new(1, Arc::clone(&heart_data), &args.input_csv);
        sensor_task.run(sensor_tx, interrupt_prob, &log_file).await;
    });

    log::info!("Sistema de Monitoramento Cardíaco iniciado");
    log::info!("Arquivo de log: {}", args.log_file);

    // Aguarda sensor terminar de ler todo o CSV
    run_handle.await.expect("sensor task panicked");

    // Encerra escalonador após leitura completa
    scheduler.shutdown().await?;
    log::info!("Leitura do CSV concluída, escalonador encerrado");

    Ok(())
}
