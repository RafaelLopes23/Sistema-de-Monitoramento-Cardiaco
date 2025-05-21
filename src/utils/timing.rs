use std::fmt;
use std::time::{Duration, Instant, SystemTime};

/// Estrutura para representar timestamps com precisão
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    /// Número de microssegundos desde o UNIX epoch
    microseconds: u64,
}

impl Timestamp {
    /// Cria um novo timestamp a partir do tempo do sistema
    pub fn now() -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));
        
        Self {
            microseconds: now.as_micros() as u64,
        }
    }
    
    /// Cria um timestamp a partir de um Instant
    pub fn from(instant: Instant) -> Self {
        // Convertemos para o tempo atual adicionando uma diferença
        let now = SystemTime::now();
        let now_instant = Instant::now();
        
        // Calcular diferença entre o Instant fornecido e o atual
        let diff = if instant <= now_instant {
            now_instant.duration_since(instant)
        } else {
            Duration::from_secs(0)
        };
        
        // Ajustar o tempo do sistema
        let adjusted_time = now
            .checked_sub(diff)
            .unwrap_or(now);
        
        let micros = adjusted_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_micros() as u64;
        
        Self {
            microseconds: micros,
        }
    }
    
    /// Retorna a diferença em duração entre dois timestamps
    pub fn duration_since(&self, earlier: &Self) -> Duration {
        if self.microseconds > earlier.microseconds {
            Duration::from_micros(self.microseconds - earlier.microseconds)
        } else {
            Duration::from_micros(0)
        }
    }
    
    /// Converte para DateTime do Chrono
    pub fn to_datetime(&self) -> chrono::DateTime<chrono::Utc> {
        let secs = (self.microseconds / 1_000_000) as i64;
        let nsecs = ((self.microseconds % 1_000_000) * 1000) as u32;
        
        chrono::DateTime::from_timestamp(secs, nsecs)
            .unwrap_or_else(|| chrono::Utc::now())
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_datetime().format("%Y-%m-%d %H:%M:%S%.6f")
        )
    }
}

/// Utilitário para medir execução de código
pub struct ExecutionTimer {
    start: Instant,
    name: String,
}

impl ExecutionTimer {
    /// Cria um novo timer
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
        }
    }
    
    /// Encerra e retorna a duração
    pub fn stop(self) -> Duration {
        let duration = self.start.elapsed();
        log::debug!("{} executou em {:?}", self.name, duration);
        duration
    }
}

impl Drop for ExecutionTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        log::debug!("{} finalizou em {:?}", self.name, duration);
    }
}