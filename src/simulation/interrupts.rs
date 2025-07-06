use crate::kernel::scheduler::SchedulerMessage;
use chrono::Local;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

/// Simulador de interrupções
pub struct InterruptSimulator {
    /// Canal para enviar mensagens ao escalonador
    scheduler_tx: Sender<SchedulerMessage>,
    /// Probabilidade de interrupção a cada check (0.0 - 1.0)
    probability: f64,
    /// Intervalo entre verificações
    check_interval: Duration,
    /// Flag para controlar a execução
    running: bool,
}

impl InterruptSimulator {
    /// Cria um novo simulador de interrupções
    pub fn new(
        scheduler_tx: Sender<SchedulerMessage>,
        probability: f64,
        check_interval: Duration,
    ) -> Self {
        Self {
            scheduler_tx,
            probability,
            check_interval,
            running: false,
        }
    }

    /// Inicia o simulador
    pub fn start(&mut self) {
        if self.running {
            return;
        }
        self.running = true;
        let tx = self.scheduler_tx.clone();
        let prob = self.probability;
        let interval = self.check_interval;

        // Iniciar thread para simular interrupções aleatórias
        tokio::spawn(async move {
            loop {
                // Esperar intervalo antes da próxima verificação
                tokio::time::sleep(interval).await;
                
                // Verificar se deve gerar uma interrupção
                if rand::random::<f64>() < prob {
                    // Gerar um tipo aleatório de interrupção
                    let interrupt_type = match rand::random::<u8>() % 4 {
                        0 => "🔌 Sensor desconectado",
                        1 => "🔋 Bateria baixa",
                        2 => "📡 Interferência de sinal",
                        _ => "⚠️ Falha de hardware",
                    };

                    // Log de interrupção simulada com timestamp
                    log::warn!("[{}] 🚨 Interrupção simulada: {}", Local::now().format("%H:%M:%S"), interrupt_type);

                    // Enviar interrupção para o escalonador
                    if tx
                        .send(SchedulerMessage::Interrupt(interrupt_type.to_string()))
                        .await
                        .is_err()
                    {
                        break; // Canal fechado, sair do loop
                    }
                }
            }
        });
    }

    /// Força a geração de uma interrupção específica
    pub async fn force_interrupt(&self, description: &str) {
        self.scheduler_tx
            .send(SchedulerMessage::Interrupt(description.to_string()))
            .await
            .ok();
    }

    /// Para o simulador
    pub fn stop(&mut self) {
        self.running = false;
    }
}
