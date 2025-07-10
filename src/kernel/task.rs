use std::fmt;
use std::pin::Pin;
use std::future::Future;
use std::time::{Duration, Instant};

/// Identificador único para cada tarefa
pub type TaskId = u32;

/// Prioridades de tarefas
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

/// Tipos de tarefas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    /// Tarefa periódica com período fixo
    Periodic { period: Duration },
    /// Tarefa aperiódica acionada por eventos
    Aperiodic,
}

/// Trait para objetos que podem ser executados como tarefas
pub trait Executable {
    /// Executa a tarefa, retornando um futuro que representa a operação assíncrona
    fn execute<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}

/// Estrutura que representa uma tarefa no sistema
pub struct Task {
    /// Identificador único da tarefa
    pub id: TaskId,
    /// Nome descritivo da tarefa
    pub name: String,
    /// Tipo da tarefa (periódica ou aperiódica)
    pub task_type: TaskType,
    /// Prioridade da tarefa
    pub priority: TaskPriority,
    /// Deadline relativo ao início da execução
    pub deadline: Duration,
    /// Pior caso de tempo de execução estimado
    pub wcet: Duration,
    /// Momento em que a tarefa foi criada
    pub created_at: Instant,
    /// Momento em que a tarefa começou a executar
    pub started_at: Option<Instant>,
    /// Momento em que a tarefa completou a execução
    pub completed_at: Option<Instant>,
    /// Possíveis dependências (IDs de tarefas que devem ser concluídas antes)
    pub dependencies: Vec<TaskId>,
    /// Executável da tarefa
    pub executable: Box<dyn Executable + Send>,
}

impl Task {
    /// Cria uma nova tarefa com os parâmetros especificados
    pub fn new(
        id: TaskId,
        name: impl Into<String>,
        task_type: TaskType,
        priority: TaskPriority,
        deadline: Duration,
        wcet: Duration,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            task_type,
            priority,
            deadline,
            wcet,
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
            dependencies: Vec::new(),
            executable: Box::new(NoOpExecutable),
        }
    }

    /// Cria uma tarefa periódica com executável
    pub fn new_periodic(
        id: TaskId,
        name: impl Into<String>,
        priority: TaskPriority,
        deadline: Duration,
        wcet: Duration,
        period: Duration,
        executable: Box<dyn Executable + Send>,
    ) -> Self {
        Task {
            id,
            name: name.into(),
            task_type: TaskType::Periodic { period },
            priority,
            deadline,
            wcet,
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
            dependencies: Vec::new(),
            executable,
        }
    }

    /// Cria uma tarefa aperiódica com executável
    pub fn new_aperiodic(
        id: TaskId,
        name: impl Into<String>,
        priority: TaskPriority,
        deadline: Duration,
        wcet: Duration,
        executable: Box<dyn Executable + Send>,
    ) -> Self {
        Task {
            id,
            name: name.into(),
            task_type: TaskType::Aperiodic,
            priority,
            deadline,
            wcet,
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
            dependencies: Vec::new(),
            executable,
        }
    }

    /// Define dependências para esta tarefa
    pub fn with_dependencies(mut self, deps: Vec<TaskId>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Verifica se a tarefa está pronta para execução
    pub fn is_ready(&self, completed_tasks: &[TaskId]) -> bool {
        self.dependencies
            .iter()
            .all(|dep| completed_tasks.contains(dep))
    }

    /// Marca a tarefa como iniciada
    pub fn start(&mut self) {
        self.started_at = Some(Instant::now());
    }

    /// Marca a tarefa como concluída
    pub fn complete(&mut self) {
        self.completed_at = Some(Instant::now());
    }

    /// Verifica se a tarefa foi concluída dentro do deadline
    pub fn met_deadline(&self) -> Option<bool> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => {
                let execution_time = end - start;
                Some(execution_time <= self.deadline)
            }
            _ => None,
        }
    }

    /// Calcula o tempo de resposta da tarefa
    pub fn response_time(&self) -> Option<Duration> {
        match (self.created_at, self.completed_at) {
            (created, Some(end)) => Some(end - created),
            _ => None,
        }
    }
}

// Manual Clone and Debug for Task (excluding executable state)
impl Clone for Task {
    fn clone(&self) -> Self {
        Task {
            id: self.id,
            name: self.name.clone(),
            task_type: self.task_type,
            priority: self.priority,
            deadline: self.deadline,
            wcet: self.wcet,
            created_at: self.created_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
            dependencies: self.dependencies.clone(),
            executable: Box::new(NoOpExecutable),
        }
    }
}
impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("task_type", &self.task_type)
            .field("priority", &self.priority)
            .field("deadline", &self.deadline)
            .field("wcet", &self.wcet)
            .field("dependencies", &self.dependencies)
            .finish()
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let task_type = match self.task_type {
            TaskType::Periodic { period } => {
                format!("Periódica (Período: {:?})", period)
            }
            TaskType::Aperiodic => "Aperiódica".to_string(),
        };

        write!(
            f,
            "Tarefa #{} \"{}\" - Tipo: {}, Prioridade: {:?}, Deadline: {:?}, WCET: {:?}",
            self.id, self.name, task_type, self.priority, self.deadline, self.wcet
        )
    }
}

/// Executor dummy para tarefas sem lógica associada
struct NoOpExecutable;
impl Executable for NoOpExecutable {
    fn execute<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
}
