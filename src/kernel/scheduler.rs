use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::Reverse;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time;

use super::task::{Task, TaskId, TaskPriority, TaskType};
use crate::utils::timing::Timestamp;

/// Mensagens que podem ser enviadas para o scheduler
#[derive(Debug)]
pub enum SchedulerMessage {
    /// Adiciona uma nova tarefa ao scheduler
    AddTask(Task),
    /// Notifica que uma tarefa foi concluída
    TaskCompleted(TaskId),
    /// Sinaliza uma interrupção de hardware
    Interrupt(String),
    /// Encerra o scheduler
    Shutdown,
}

/// Resultado da execução de uma tarefa
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub id: TaskId,
    pub name: String,
    pub started_at: Timestamp,
    pub completed_at: Timestamp,
    pub deadline_met: bool,
    pub execution_time: Duration,
}

/// Escalonador baseado em prioridades fixas com preempção
pub struct Scheduler {
    /// Canal para enviar mensagens ao scheduler
    tx: Sender<SchedulerMessage>,
    /// Canal para receber resultados de execução de tarefas
    results_rx: Arc<Mutex<Receiver<TaskResult>>>,
    /// Mapa de tarefas periódicas para reescalonar
    periodic_tasks: Arc<Mutex<HashMap<TaskId, Task>>>,
    /// Tarefas concluídas (para verificar dependências)
    completed_tasks: Arc<Mutex<Vec<TaskId>>>,
}

impl Scheduler {
    /// Cria um novo escalonador
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        let (results_tx, results_rx) = mpsc::channel(100);
        
        let periodic_tasks = Arc::new(Mutex::new(HashMap::new()));
        let completed_tasks = Arc::new(Mutex::new(Vec::new()));
        
        let periodic_tasks_clone = Arc::clone(&periodic_tasks);
        let completed_tasks_clone = Arc::clone(&completed_tasks);

        // Inicia o loop do escalonador em uma thread separada
        tokio::spawn(Self::scheduler_loop(
            rx, 
            results_tx, 
            periodic_tasks_clone,
            completed_tasks_clone
        ));

        Self {
            tx,
            results_rx: Arc::new(Mutex::new(results_rx)),
            periodic_tasks,
            completed_tasks,
        }
    }

    /// Adiciona uma nova tarefa ao escalonador
    pub async fn add_task(&self, task: Task) -> Result<(), String> {
        // Armazenar tarefas periódicas para reescalonar
        if let TaskType::Periodic { .. } = task.task_type {
            let mut periodic_tasks = self.periodic_tasks.lock().unwrap();
            periodic_tasks.insert(task.id, task.clone());
        }
        
        self.tx.send(SchedulerMessage::AddTask(task))
            .await
            .map_err(|e| format!("Falha ao adicionar tarefa: {}", e))
    }

    /// Notifica o escalonador que uma tarefa foi concluída
    pub async fn task_completed(&self, task_id: TaskId) -> Result<(), String> {
        // Adicionar à lista de tarefas concluídas
        {
            let mut completed = self.completed_tasks.lock().unwrap();
            if !completed.contains(&task_id) {
                completed.push(task_id);
            }
        }
        
        self.tx.send(SchedulerMessage::TaskCompleted(task_id))
            .await
            .map_err(|e| format!("Falha ao notificar conclusão: {}", e))
    }

    /// Simula uma interrupção de hardware
    pub async fn simulate_interrupt(&self, description: &str) -> Result<(), String> {
        self.tx.send(SchedulerMessage::Interrupt(description.to_string()))
            .await
            .map_err(|e| format!("Falha ao simular interrupção: {}", e))
    }

    /// Encerra o escalonador
    pub async fn shutdown(&self) -> Result<(), String> {
        self.tx.send(SchedulerMessage::Shutdown)
            .await
            .map_err(|e| format!("Falha ao encerrar escalonador: {}", e))
    }

    /// Recebe resultados de execução de tarefas
    pub async fn receive_result(&self) -> Option<TaskResult> {
        let mut rx = self.results_rx.lock().unwrap();
        rx.try_recv().ok()
    }

    /// Loop principal do escalonador
    async fn scheduler_loop(
        mut rx: Receiver<SchedulerMessage>,
        results_tx: Sender<TaskResult>,
        periodic_tasks: Arc<Mutex<HashMap<TaskId, Task>>>,
        completed_tasks: Arc<Mutex<Vec<TaskId>>>,
    ) {
        // Fila de tarefas prontas para executar, ordenadas por prioridade
        let mut ready_queue: BinaryHeap<(TaskPriority, Reverse<Instant>, TaskId)> = BinaryHeap::new();
        
        // Armazenamento de tarefas
        let mut tasks: HashMap<TaskId, Task> = HashMap::new();
        
        // Tarefa em execução
        let mut current_task: Option<Task> = None;
        
        // Contador para gerar IDs de tarefas
        let mut next_id: TaskId = 1;

        // Loop principal do escalonador
        loop {
            tokio::select! {
                // Processar mensagens enviadas ao escalonador
                Some(msg) = rx.recv() => {
                    match msg {
                        SchedulerMessage::AddTask(task) => {
                            log::info!("Tarefa adicionada: {}", task.name);
                            let task_id = task.id;
                            let task_priority = task.priority;
                            
                            // Verificar se a tarefa está pronta para execução
                            let completed = completed_tasks.lock().unwrap();
                            let is_ready = task.is_ready(&completed);
                            drop(completed);
                            
                            if is_ready {
                                // Adicionar à fila de prontos
                                ready_queue.push((task_priority, Reverse(Instant::now()), task_id));
                            }
                            
                            tasks.insert(task_id, task);
                            
                            // Verificar preempção
                            if let Some(ref current) = current_task {
                                if task_priority > current.priority {
                                    // Preempção: interromper tarefa atual se a nova tiver maior prioridade
                                    if let Some(mut interrupted) = current_task.take() {
                                        log::info!("Preempção: {} interrompida", interrupted.name);
                                        // Recolocar a tarefa interrompida na fila
                                        ready_queue.push((interrupted.priority, Reverse(interrupted.created_at), interrupted.id));
                                    }
                                }
                            }
                        },
                        SchedulerMessage::TaskCompleted(task_id) => {
                            if let Some(mut task) = tasks.remove(&task_id) {
                                // Marcar a tarefa como concluída
                                task.complete();
                                log::info!("Tarefa concluída: {}", task.name);
                                
                                // Enviar resultado
                                if let (Some(start), Some(end)) = (task.started_at, task.completed_at) {
                                    let execution_time = end - start;
                                    let deadline_met = execution_time <= task.deadline;
                                    
                                    results_tx.send(TaskResult {
                                        id: task.id,
                                        name: task.name.clone(),
                                        started_at: Timestamp::from(start),
                                        completed_at: Timestamp::from(end),
                                        deadline_met,
                                        execution_time,
                                    }).await.ok();
                                    
                                    // Se for uma tarefa periódica, reescalonar
                                    if let TaskType::Periodic { period } = task.task_type {
                                        let mut periodic = periodic_tasks.lock().unwrap();
                                        if let Some(template) = periodic.get(&task_id) {
                                            // Criar nova instância
                                            let mut new_task = template.clone();
                                            new_task.id = next_id;
                                            next_id += 1;
                                            
                                            // Reescalonar após o período
                                            let new_task_id = new_task.id;
                                            let new_task_priority = new_task.priority;
                                            
                                            // Adicionar tarefa ao mapa
                                            tasks.insert(new_task_id, new_task);
                                            
                                            // Agendar próxima execução
                                            let tx = rx.clone();
                                            tokio::spawn(async move {
                                                time::sleep(period).await;
                                                tx.send(SchedulerMessage::AddTask(
                                                    Task::new(
                                                        new_task_id,
                                                        format!("{} (reescalonada)", template.name),
                                                        TaskType::Periodic { period },
                                                        new_task_priority,
                                                        template.deadline,
                                                        template.wcet,
                                                    )
                                                )).await.ok();
                                            });
                                        }
                                    }
                                }
                                
                                // Verificar tarefas que estavam esperando esta concluir
                                for (_, waiting_task) in tasks.iter() {
                                    if waiting_task.dependencies.contains(&task_id) {
                                        let completed = completed_tasks.lock().unwrap();
                                        if waiting_task.is_ready(&completed) {
                                            ready_queue.push((
                                                waiting_task.priority,
                                                Reverse(waiting_task.created_at),
                                                waiting_task.id
                                            ));
                                        }
                                        drop(completed);
                                    }
                                }
                                
                                // Limpar referência à tarefa atual
                                if current_task.as_ref().map(|t| t.id) == Some(task_id) {
                                    current_task = None;
                                }
                            }
                        },
                        SchedulerMessage::Interrupt(description) => {
                            log::warn!("Interrupção recebida: {}", description);
                            
                            // Criar uma tarefa aperiódica de alta prioridade para lidar com a interrupção
                            let interrupt_task = Task::new(
                                next_id,
                                format!("Interrupção: {}", description),
                                TaskType::Aperiodic,
                                TaskPriority::Critical,
                                Duration::from_millis(50),  // Deadline curto para interrupções
                                Duration::from_millis(20),  // WCET estimado
                            );
                            
                            next_id += 1;
                            
                            // Adicionar à fila com prioridade máxima
                            let task_id = interrupt_task.id;
                            tasks.insert(task_id, interrupt_task);
                            ready_queue.push((TaskPriority::Critical, Reverse(Instant::now()), task_id));
                            
                            // Preempção forçada para interrupção
                            if current_task.is_some() {
                                if let Some(interrupted) = current_task.take() {
                                    log::info!("Preempção por interrupção: {} interrompida", interrupted.name);
                                    ready_queue.push((interrupted.priority, Reverse(interrupted.created_at), interrupted.id));
                                }
                            }
                        },
                        SchedulerMessage::Shutdown => {
                            log::info!("Escalonador encerrando");
                            break;
                        }
                    }
                }
                
                // Se não há tarefa em execução, pegar a próxima da fila
                _ = time::sleep(Duration::from_millis(10)), if current_task.is_none() => {
                    if let Some((_, _, task_id)) = ready_queue.pop() {
                        if let Some(mut task) = tasks.remove(&task_id) {
                            // Começar execução
                            log::info!("Iniciando execução: {}", task.name);
                            task.start();
                            
                            // Simular execução da tarefa
                            let task_id = task.id;
                            let wcet = task.wcet;
                            let task_tx = rx.clone();
                            
                            current_task = Some(task);
                            
                            // Spawn de uma tarefa para simular a execução
                            tokio::spawn(async move {
                                // Simular tempo de execução (pode ser aleatório dentro de limites)
                                time::sleep(wcet).await;
                                
                                // Notificar conclusão
                                task_tx.send(SchedulerMessage::TaskCompleted(task_id)).await.ok();
                            });
                        }
                    }
                }
            }
        }
    }
}