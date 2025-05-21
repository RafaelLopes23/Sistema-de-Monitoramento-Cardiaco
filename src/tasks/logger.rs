use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use std::fs::OpenOptions;
use std::io::Write;

use crate::kernel::task::{Task, TaskId, TaskPriority, TaskType};
use crate::tasks::sensor::HeartbeatData;

/// Tarefa de registro de logs
pub struct LoggerTask {
    /// ID da tarefa
    pub id: TaskId,
    /// Estado compartilhado para ler dados do sensor
    pub sensor_data: Arc<Mutex<HeartbeatData>>,
    /// Caminho do arquivo de log
    pub log_file: String,
}

impl LoggerTask {
    /// Cria uma nova tarefa de registro
    pub fn new(id: TaskId, sensor_data: Arc<Mutex<HeartbeatData>>, log_file: &str) -> Self {
        Self {
            id,
            sensor_data,
            log_file: log_file.to_string(),
        }
    }

    /// Retorna uma definição de tarefa para o escalonador
    pub fn get_task_definition(&self) -> Task {
        Task::new(
            self.id,
            "Registro de Logs",
            TaskType::Periodic {
                period: Duration::from_secs(1),
            },
            TaskPriority::Low,
            Duration::from_millis(200), // Deadline: 200ms
            Duration::from_millis(30),  // WCET: 30ms
        )
        .with_dependencies(vec![1]) // Depende da tarefa de sensor (ID 1)
    }

    /// Executa a lógica da tarefa
    pub async fn execute(&mut self) {
        // Ler dados do sensor
        let data = self.sensor_data.lock().await;
        
        // Formatar a entrada de log
        let log_entry = format!(
            "{},{},{}\n",
            data.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            data.bpm,
            if data.is_anomaly { "ANOMALY" } else { "NORMAL" }
        );
        
        // Escrever no arquivo de log
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
        {
            Ok(mut file) => {
                if let Err(e) = file.write_all(log_entry.as_bytes()) {
                    log::error!("Erro ao escrever no arquivo de log: {}", e);
                }
            }
            Err(e) => {
                log::error!("Erro ao abrir arquivo de log: {}", e);
            }
        }
        
        log::info!("Log registrado: BPM={} às {}", 
                   data.bpm, 
                   data.timestamp.format("%H:%M:%S"));
    }
}