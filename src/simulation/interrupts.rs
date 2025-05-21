use std::time::Duration;
use rand::Rng;
use tokio::sync::mpsc::Sender;
use crate::kernel::scheduler::SchedulerMessage;

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
            let mut rng = rand::thread_rng();
            
            while tx.capacity() > 0 { // Verificar se o canal ainda está aberto
                // Verificar se deve gerar uma interrupção
                if rng.gen::<f64>() < prob {
                    // Gerar um tipo aleatório de interrupção
                    let interrupt_type = match rng.gen_range(0..3) {
                        0 => "Sensor desconectado",
                        1 => "Bateria baixa",
                        _ => "Interferência de sinal",
                    };
                    
                    log::debug!("Simulando interrupção: {}", interrupt_type);
                    
                    // Enviar interrupção para o escalonador
                    tx.send(SchedulerMessage::Interrupt(interrupt_type.to_string()))
                        .await
                        .ok();
                }
                
                // Esperar intervalo antes da próxima verificação
                tokio::time::sleep(interval).await;
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