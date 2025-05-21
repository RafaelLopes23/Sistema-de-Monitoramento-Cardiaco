use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::kernel::task::{Task, TaskId, TaskPriority, TaskType};
use crate::simulation::heart_data::HeartbeatGenerator;

/// Estrutura de dados compartilhada do sensor
#[derive(Debug, Clone)]
pub struct HeartbeatData {
    pub bpm: u32,
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub is_anomaly: bool,
}

/// Tarefa de coleta de sinais cardíacos
pub struct HeartbeatSensorTask {
    /// ID da tarefa
    pub id: TaskId,
    /// Estado compartilhado para armazenar as leituras do sensor
    pub shared_data: Arc<Mutex<HeartbeatData>>,
    /// Gerador de batimentos cardíacos simulados
    pub generator: HeartbeatGenerator,
}

impl HeartbeatSensorTask {
    /// Cria uma nova tarefa de sensor
    pub fn new(id: TaskId, shared_data: Arc<Mutex<HeartbeatData>>) -> Self {
        Self {
            id,
            shared_data,
            generator: HeartbeatGenerator::new(70, 5), // Média 70 BPM, variação 5
        }
    }

    /// Retorna uma definição de tarefa para o escalonador
    pub fn get_task_definition(&self) -> Task {
        Task::new(
            self.id,
            "Sensor de Batimentos",
            TaskType::Periodic {
                period: Duration::from_millis(500),
            },
            TaskPriority::Medium,
            Duration::from_millis(100), // Deadline: 100ms
            Duration::from_millis(50),  // WCET: 50ms
        )
    }

    /// Executa a lógica da tarefa
    pub async fn execute(&mut self) {
        // Simular leitura do sensor
        let bpm = self.generator.next_value();
        let timestamp = chrono::Local::now();
        
        // Detectar anomalia
        let is_anomaly = bpm < 60 || bpm > 120;
        
        // Atualizar dados compartilhados
        let mut data = self.shared_data.lock().await;
        *data = HeartbeatData {
            bpm,
            timestamp,
            is_anomaly,
        };
        
        if is_anomaly {
            log::warn!("Anomalia detectada: {} BPM", bpm);
        } else {
            log::info!("Batimento normal: {} BPM", bpm);
        }
    }
}