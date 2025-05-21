use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::kernel::task::{Task, TaskId, TaskPriority, TaskType};
use crate::tasks::sensor::HeartbeatData;

/// Tarefa de alarme de emergência
pub struct EmergencyAlarmTask {
    /// ID da tarefa
    pub id: TaskId,
    /// Estado compartilhado para ler dados do sensor
    pub sensor_data: Arc<Mutex<HeartbeatData>>,
    /// Contador de alarmes disparados
    pub alarm_count: u32,
}

impl EmergencyAlarmTask {
    /// Cria uma nova tarefa de alarme
    pub fn new(id: TaskId, sensor_data: Arc<Mutex<HeartbeatData>>) -> Self {
        Self {
            id,
            sensor_data,
            alarm_count: 0,
        }
    }

    /// Retorna uma definição de tarefa para o escalonador
    pub fn get_task_definition(&self) -> Task {
        Task::new(
            self.id,
            "Alarme de Emergência",
            TaskType::Aperiodic,
            TaskPriority::Critical,
            Duration::from_millis(50), // Deadline: 50ms
            Duration::from_millis(20), // WCET: 20ms
        )
    }

    /// Executa a lógica da tarefa
    pub async fn execute(&mut self) {
        // Ler dados do sensor
        let data = self.sensor_data.lock().await;
        
        if data.is_anomaly {
            self.alarm_count += 1;
            
            // Simulação de um alarme
            log::error!("⚠️ ALARME DE EMERGÊNCIA! BPM: {} às {}", 
                       data.bpm, 
                       data.timestamp.format("%H:%M:%S"));
            
            // Simulação de uma notificação externa (ex.: enviar SMS, acionar sirene)
            log::warn!("Enviando notificação para equipe médica. Alarme #{}", self.alarm_count);
            
            // Simular tempo de processamento do alarme
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
    }
}