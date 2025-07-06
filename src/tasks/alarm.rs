use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
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
    /// Canal opcional para enviar logs à GUI
    pub log_sender: Option<UnboundedSender<String>>,
}

impl EmergencyAlarmTask {
    /// Cria uma nova tarefa de alarme
    pub fn new(id: TaskId, sensor_data: Arc<Mutex<HeartbeatData>>) -> Self {
        Self { id, sensor_data, alarm_count: 0, log_sender: None }
    }
    /// Define canal para GUI receber logs
    pub fn set_log_sender(&mut self, sender: UnboundedSender<String>) {
        self.log_sender = Some(sender);
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
            log::error!("⚠️ ALARME DE EMERGÊNCIA: {:.3} mV às {}", data.mv, data.timestamp.format("%H:%M:%S"));
            if let Some(tx) = &self.log_sender {
                tx.send(format!("⚠️ EMERGENCY ALARM: {:.3} mV at {}", data.mv, data.timestamp.format("%H:%M:%S"))).ok();
            }

            // Simulação de uma notificação externa (ex.: enviar SMS, acionar sirene)
            log::warn!(
                "Enviando notificação para equipe médica. Alarme #{}",
                self.alarm_count
            );

            // Simular tempo de processamento do alarme
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
    }
}
