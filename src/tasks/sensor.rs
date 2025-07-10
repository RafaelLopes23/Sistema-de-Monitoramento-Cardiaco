use std::fs::File;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use csv::{Reader, StringRecordsIntoIter};
use tokio::time::sleep;
use crate::kernel::task::{Task, TaskId, TaskPriority, TaskType};
use crate::kernel::scheduler::SchedulerMessage;
use tokio::sync::mpsc::Sender;
use std::fs::OpenOptions;
use std::io::Write;
use rand::random;
use chrono::Local;
use std::time::Duration as StdDuration;
use tokio::sync::mpsc::UnboundedSender;
use std::pin::Pin;
use std::future::Future;
use crate::kernel::task::Executable;

/// Estrutura de dados compartilhada do sensor
#[derive(Debug, Clone)]
pub struct HeartbeatData {
    pub mv: f32,
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub is_anomaly: bool,
}

/// Tarefa de coleta de sinais cardíacos
pub struct HeartbeatSensorTask {
    /// ID da tarefa
    pub id: TaskId,
    /// Estado compartilhado para armazenar as leituras do sensor
    pub shared_data: Arc<Mutex<HeartbeatData>>,
    /// Iterador sobre registros do CSV de ECG
    pub csv_iter: StringRecordsIntoIter<File>,
    /// Canal opcional para enviar logs a GUI
    pub log_sender: Option<UnboundedSender<String>>,
}

impl HeartbeatSensorTask {
    /// Cria uma nova tarefa de sensor
    pub fn new(id: TaskId, shared_data: Arc<Mutex<HeartbeatData>>, csv_path: &str) -> Self {
        let file = File::open(csv_path).expect("falha ao abrir CSV de ECG");
        let rdr = Reader::from_reader(file);
        let iter = rdr.into_records();
        Self { id, shared_data, csv_iter: iter, log_sender: None }
    }
    /// Define canal para GUI receber logs
    pub fn set_log_sender(&mut self, sender: UnboundedSender<String>) {
        self.log_sender = Some(sender);
    }

    /// Retorna uma definição de tarefa para o escalonador
    pub fn get_task_definition(&self) -> Task {
        Task::new(
            self.id,
            "Sensor de ECG",
            TaskType::Periodic { period: Duration::from_millis(4) },  // 250Hz ~4ms
            TaskPriority::Medium,
            Duration::from_millis(4),  // deadline igual ao período
            Duration::from_millis(1),  // WCET estimado
        )
    }

    /// Executa a lógica da tarefa
    pub async fn execute(&mut self) {
        // Lê próxima linha do CSV
        if let Some(Ok(record)) = self.csv_iter.next() {
            let mv = record.get(1)
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);
            let timestamp = chrono::Local::now();
            let is_anomaly = mv > 10.0;
            // Aviso chamativo para equipe médica em caso de anomalia
            if is_anomaly {
                log::error!("🚨🚑 ALARME MÉDICO! ANOMALIA DETECTADA: {:.3} mV às {} – enviando alerta à equipe médica 🚑🚨", mv, timestamp.format("%H:%M:%S%.3f"));
                if let Some(tx) = &self.log_sender {
                    let msg = format!("🚨 Medical Alert: {:.3} mV at {}", mv, timestamp.format("%H:%M:%S%.3f"));
                    tx.send(msg).ok();
                }
            }

            let mut data = self.shared_data.lock().await;
            *data = HeartbeatData { mv, timestamp, is_anomaly };

            if is_anomaly {
                log::warn!("Anomalia no ECG: {:.3} mV", mv);
            } else {
                log::info!("ECG normal: {:.3} mV", mv);
                if let Some(tx) = &self.log_sender {
                    tx.send(format!("ECG normal: {:.3} mV at {}", mv, timestamp.format("%H:%M:%S%.3f"))).ok();
                }
            }
        } else {
            log::info!("CSV de ECG finalizado");
        }
    }

    /// Loop de leitura do CSV em tempo real baseado no campo de timestamp
    pub async fn run(mut self, scheduler_tx: Sender<SchedulerMessage>, interrupt_prob: f64, log_file: &str) {
        // Abrir arquivo de log de ECG (append)
        let mut file = OpenOptions::new().create(true).append(true).open(log_file)
            .expect("Não foi possível abrir arquivo de log");
        // Cabeçalho (inclui coluna event e horário real)
        file.write_all(b"elapsed,ecg_mv,status,event,real_ts\n").ok();
        let mut prev: Option<StdDuration> = None;
        while let Some(Ok(record)) = self.csv_iter.next() {
            // Extrair string do timestamp e valor mV
            let ts_str = record.get(0).unwrap_or("0:00.000");
            let mv = record.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
            // Parsing manual do formato M:SS.mmm
            let mut parts = ts_str.split(':');
            let min = parts.next().unwrap_or("0").parse::<u64>().unwrap_or(0);
            let rest = parts.next().unwrap_or("0.000");
            let mut subs = rest.split('.');
            let sec = subs.next().unwrap_or("0").parse::<u64>().unwrap_or(0);
            let ms = subs.next().unwrap_or("0").parse::<u64>().unwrap_or(0);
            let cur = StdDuration::from_secs(min * 60 + sec) + StdDuration::from_millis(ms);
            // Aguarda diferença entre amostras
            let dt = if let Some(prev_dur) = prev {
                let delta = cur.checked_sub(prev_dur).unwrap_or_default();
                if delta > StdDuration::ZERO { sleep(delta).await; }
                delta
            } else {
                sleep(cur).await;
                cur
            };
            prev = Some(cur);
            let timestamp = Local::now();
            let is_anomaly = mv > 10.0;
            // Aviso chamativo para equipe médica em caso de anomalia
            if is_anomaly {
                log::error!("🚨🚑 ALARME MÉDICO! ANOMALIA DETECTADA: {:.3} mV em {} – enviando alerta à equipe médica 🚑🚨", mv, ts_str);
            }
            {
                let mut data = self.shared_data.lock().await;
                *data = HeartbeatData { mv, timestamp, is_anomaly };
            }
            // Gravar linha de leitura, incluindo event em caso de anomalia
            let status = if is_anomaly { "ANOMALY" } else { "NORMAL" };
            let event_field = if is_anomaly { "🚨Medical Alert🚨" } else { "" };
            let line = format!("{},{:.3},{},{},{}\n",
                ts_str,
                mv,
                status,
                event_field,
                timestamp.format("%H:%M:%S%.3f")
            );
            file.write_all(line.as_bytes()).ok();
            // Feedback no console
            log::info!("Leitura ECG: {:.3} mV @{} (real @{})", mv, ts_str, timestamp.format("%H:%M:%S%.3f"));
            if let Some(tx) = &self.log_sender {
                tx.send(format!("Leitura ECG: {:.3} mV @{}", mv, ts_str)).ok();
            }
            // Checagem única de interrupção para este intervalo
            let p = interrupt_prob * dt.as_secs_f64();
            if random::<f64>() < p {
                let interrupt_type = match random::<u8>() % 4 {
                    0 => "🔋 Bateria baixa",
                    1 => "📡 Interferência de sinal",
                    2 => "⚠️ Falha de hardware",
                    _ => "🔌 Sensor desconectado",
                };
                let desc = format!("{} at {}", interrupt_type, ts_str);
                // Gravar evento de interrupção no CSV
                let ev_line = format!("{},{:.3},{},{},{}\n", ts_str, mv, status, interrupt_type, timestamp.format("%H:%M:%S%.3f"));
                file.write_all(ev_line.as_bytes()).ok();
                scheduler_tx.send(SchedulerMessage::Interrupt(desc)).await.ok();
                log::info!("Interrupção randômica disparada: {}", interrupt_type);
                if let Some(tx) = &self.log_sender {
                    tx.send(format!("Interrupção randômica: {} at {}", interrupt_type, ts_str)).ok();
                }
            }
        }
        log::info!("CSV de ECG finalizado");
    }
}

impl Executable for HeartbeatSensorTask {
    fn execute<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            self.execute().await;
        })
    }
}
