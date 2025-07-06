use rand::thread_rng;
use rand_distr::{Distribution, Normal};

/// Gerador de dados de ECG simulados em mV
pub struct HeartbeatGenerator {
    /// Média em mV
    mean: f64,
    /// Desvio padrão em mV
    std_dev: f64,
    /// Distribuição normal para gerar valores
    distribution: Normal<f64>,
    /// Gerador de números aleatórios
    rng: rand::rngs::ThreadRng,
    /// Contador para gerar anomalias periódicas
    anomaly_counter: u32,
}

impl HeartbeatGenerator {
    /// Cria um novo gerador de ECG com média e desvio padrão em mV
    pub fn new(mean_mv: f32, std_dev: f32) -> Self {
        let mean = mean_mv as f64;
        let std_dev = std_dev as f64;

        Self {
            mean,
            std_dev,
            distribution: Normal::new(mean, std_dev).unwrap(),
            rng: thread_rng(),
            anomaly_counter: 0,
        }
    }

    /// Gera o próximo valor de ECG em mV
    pub fn next_value(&mut self) -> f32 {
        self.anomaly_counter = self.anomaly_counter.wrapping_add(1);

        // A cada 20 leituras, gerar uma anomalia >10 mV ou baixa (<5 mV)
        if self.anomaly_counter >= 20 {
            self.anomaly_counter = 0;
            return if rand::random::<bool>() {
                12.0 // anomalia alta
            } else {
                4.0 // anomalia baixa
            };
        }

        // Valor normal seguindo distribuição gaussiana
        let mut val = self.distribution.sample(&mut self.rng);

        // Garantir valor não-negativo
        if val < 0.0 { val = 0.0; }
        val as f32
    }

    /// Define manualmente a média em mV
    pub fn set_mean(&mut self, mean_mv: f32) {
        self.mean = mean_mv as f64;
        self.distribution = Normal::new(self.mean, self.std_dev).unwrap();
    }

    /// Define manualmente o desvio padrão em mV
    pub fn set_std_dev(&mut self, std_dev: f32) {
        self.std_dev = std_dev as f64;
        self.distribution = Normal::new(self.mean, self.std_dev).unwrap();
    }

    /// Força a geração de uma anomalia específica em mV
    pub fn force_anomaly(&mut self, high: bool) -> f32 {
        if high { 12.0 } else { 4.0 }
    }
}
