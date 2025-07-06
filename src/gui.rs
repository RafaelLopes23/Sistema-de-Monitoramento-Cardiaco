use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::runtime::Runtime;

use sistema_monitor_cardiaco::tasks::sensor::{HeartbeatSensorTask, HeartbeatData};
use sistema_monitor_cardiaco::tasks::alarm::EmergencyAlarmTask;
use sistema_monitor_cardiaco::kernel::scheduler::SchedulerMessage;

pub struct MyApp {
    // images: Vec<RetainedImage>,
    // img_idx: usize,
    // last_img_swap: Instant,
    waveform: Vec<[f64; 2]>,
    sample_x: f64,
    logs: Arc<Mutex<Vec<String>>>,
    log_rx: mpsc::UnboundedReceiver<String>,
}

impl MyApp {
    fn update(&mut self) {
        if self.waveform.len() > 1000 {
            // limitar tamanho
            let to_remove = self.waveform.len() - 1000;
            self.waveform.drain(0..to_remove);
        }
        while let Ok(line) = self.log_rx.try_recv() {
            self.logs.lock().unwrap().push(line.clone());
            if let Some(rest) = line.strip_prefix("Leitura ECG: ") {
                if let Some(val_str) = rest.split(' ').next() {
                    if let Ok(v) = val_str.parse::<f64>() {
                        self.waveform.push([self.sample_x, v]);
                        self.sample_x += 0.004;
                    }
                }
            }
        }
    }
}

fn main() {
    env_logger::init();
    let (tx, rx) = mpsc::unbounded_channel();
    let logs = Arc::new(Mutex::new(Vec::new()));
    let logs_clone = logs.clone();
    let tx_sensor = tx.clone();
    let tx_alarm = tx.clone();

    let rt = Runtime::new().unwrap();
    rt.spawn(async move {
        let heart_data = Arc::new(tokio::sync::Mutex::new(HeartbeatData { mv: 0.0, timestamp: chrono::Local::now(), is_anomaly: false }));
        // Dummy channel for scheduler messages
        let (sched_tx, _sched_rx) = mpsc::channel::<SchedulerMessage>(10);
        let mut sensor = HeartbeatSensorTask::new(1, heart_data.clone(), "ecg_input.csv");
        sensor.set_log_sender(tx_sensor);
        sensor.run(sched_tx.clone(), 0.05, "cardiac_log.csv").await;
    });
    rt.spawn(async move {
        let heart_data = Arc::new(tokio::sync::Mutex::new(HeartbeatData { mv: 0.0, timestamp: chrono::Local::now(), is_anomaly: false }));
        let mut alarm = EmergencyAlarmTask::new(2, heart_data.clone());
        alarm.set_log_sender(tx_alarm);
        alarm.execute().await;
    });

    let app = MyApp { waveform: Vec::new(), sample_x: 0.0, logs: logs_clone, log_rx: rx };
    // GUI temporariamente removida. Implementação futura em outra branch.
}
