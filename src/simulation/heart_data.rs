use rand::distributions::{Distribution, Normal};
use rand::thread_rng;

/// Gerador de dados cardíacos simulados
pub struct HeartbeatGenerator {
    /// Média de BPM
    mean: f64,
    /// Desvio padrão
    std_dev: f64,
    /// Distribuição normal para gerar valores
    distribution: Normal,
    /// Gerador de números aleatórios
    rng: rand::rngs::ThreadRng,
    /// Contador para gerar anomalias periódicas
    anomaly_counter: u32,
}

impl HeartbeatGenerator {
    /// Cria um novo gerador de batimentos
    pub fn new(mean_bpm: u32, std_dev: u32) -> Self {
        let mean = mean_bpm as f64;
        let std_dev = std_dev as f64;
        
        Self {
            mean,
            std_dev,
            distribution: Normal::new(mean, std_dev),
            rng: thread_rng(),
            anomaly_counter: 0,
        }
    }
    
    /// Gera o próximo valor de batimento cardíaco
    pub fn next_value(&mut self) -> u32 {
        self.anomaly_counter += 1;
        
        // A cada 20 leituras, gerar uma anomalia
        if self.anomaly_counter >= 20 {
            self.anomaly_counter = 0;
            
            // 50% de chance de bradicardia (batimento baixo)
            if rand::random::<bool>() {
                return 55; // Bradicardia
            } else {
                return 130; // Taquicardia
            }
        }
        
        // Valor normal seguindo distribuição gaussiana
        let mut bpm = self.distribution.sample(&mut self.rng);
        
        // Garantir que BPM seja positivo
        if bpm < 0.0 {
            bpm = 0.0;
        }
        
        bpm as u32
    }
    
    /// Define manualmente a média
    pub fn set_mean(&mut self, mean: u32) {
        self.mean = mean as f64;
        self.distribution = Normal::new(self.mean, self.std_dev);
    }
    
    /// Define manualmente o desvio padrão
    pub fn set_std_dev(&mut self, std_dev: u32) {
        self.std_dev = std_dev as f64;
        self.distribution = Normal::new(self.mean, self.std_dev);
    }
    
    /// Força a geração de uma anomalia específica
    pub fn force_anomaly(&mut self, high: bool) -> u32 {
        if high {
            130 // Taquicardia
        } else {
            55  // Bradicardia
        }
    }
}