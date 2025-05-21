mod kernel;
mod tasks;
mod simulation;
mod utils;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use clap::Parser;

use crate::kernel::scheduler::Scheduler;
use crate::tasks::sensor::{HeartbeatData, HeartbeatSensorTask};
use crate::tasks::alarm::EmergencyAlarmTask;
use crate::tasks::logger::LoggerTask;
use crate::simulation::interrupts::InterruptSimulator;

/// Sistema de Monitoramento Cardíaco em Tempo Real
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Duração da simulação em segundos
    #[arg(short, long, default_value_t = 60)]
    duration: u64,
    
    /// Arquivo para salvar logs
    #[arg(short, long, default_value = "cardiac_data.csv")]
    log_file: String,
    
    /// Média de BPM
    #[arg(short, long, default_value_t = 70)]
    mean_bpm: u32,
    
    /// Variação de BPM
    #[arg(short, long, default_value_t = 5)]
    bpm_variation: u32,
    
    /// Probabilidade de interrupções (0-100)
    #[arg(short, long, default_value_t = 10)]
    interrupt_probability: u32,
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
        bpm: args.mean_bpm,
        timestamp: chrono::Local::now(),
        is_anomaly: false,
    }));
    
    // Criar tarefas
    let mut sensor_task = HeartbeatSensorTask::new(1, Arc::clone(&heart_data));
    sensor_task.generator.set_mean(args.mean_bpm);
    sensor_task.generator.set_std_dev(args.bpm_variation);
    
    let mut alarm_task = EmergencyAlarmTask::new(2, Arc::clone(&heart_data));
    let mut logger_task = LoggerTask::new(3, Arc::clone(&heart_data), &args.log_file);
    
    // Criar cabeçalho do arquivo CSV
    std::fs::write(
        &args.log_file,
        "timestamp,bpm,status\n"
    )?;
    
    // Configurar simulador de interrupções
    let mut interrupt_simulator = InterruptSimulator::new(
        scheduler.tx.clone(),
        args.interrupt_probability as f64 / 100.0,
        Duration::from_secs(5),
    );
    interrupt_simulator.start();
    
    // Registrar tarefas no escalonador
    scheduler.add_task(sensor_task.get_task_definition()).await?;
    scheduler.add_task(alarm_task.get_task_definition()).await?;
    scheduler.add_task(logger_task.get_task_definition()).await?;
    
    log::info!("Sistema de Monitoramento Cardíaco iniciado");
    log::info!("Duração da simulação: {} segundos", args.duration);
    log::info!("Arquivo de log: {}", args.log_file);
    
    // Executar simulação pelo tempo especificado
    tokio::time::sleep(Duration::from_secs(args.duration)).await;
    
    // Forçar uma interrupção para teste
    interrupt_simulator.force_interrupt("Alarme de teste").await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Encerrar escalonador
    scheduler.shutdown().await?;
    
    log::info!("Simulação concluída");
    
    Ok(())
}