use crate::models::LlmProvider;
use crate::models::{ChatRequest, Message, ModelError, Role};
use crate::scheduler::Scheduler;
use crate::types::*;
use std::sync::Arc;

/// Overmind — 星空虫族的最高意志。
/// 理解用户意图、分裂任务、派遣子Agent（兵虫）。
pub struct Overmind {
    pub identity: AgentIdentity,
    pub scene: Option<u64>,
    pub model: String,
    /// Provider 用于主Agent 自己的推理
    provider: Option<Arc<dyn LlmProvider>>,
}

/// 子Agent（兵虫）— 执行单个任务的轻量 Agent
pub struct SubAgent {
    pub id: u64,
    pub task: Task,
    pub state: SubAgentState,
    provider: Option<Arc<dyn LlmProvider>>,
}

#[derive(Debug, Clone)]
pub struct AgentIdentity {
    pub name: String,
    pub kind: AgentKind,
}

#[derive(Debug, Clone)]
pub enum AgentKind {
    Main,
    SubAgent,
}

impl Overmind {
    pub fn new() -> Self {
        Self {
            identity: AgentIdentity { name: "Overmind".into(), kind: AgentKind::Main },
            scene: None,
            model: "gpt-4o-mini".into(),
            provider: None,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_provider(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// 分析用户输入，分裂为子任务列表
    pub async fn decompose(&self, input: &str, messages: &[Message]) -> Result<Vec<String>, ModelError> {
        let provider = self.provider.as_ref().ok_or(ModelError::ModelUnavailable("无 Provider".into()))?;

        // 构建分解提示
        let mut ctx = messages.to_vec();
        ctx.push(Message {
            role: Role::User,
            content: format!("将以下用户需求分解为可并行执行的子任务列表。每行一个子任务，不要编号。\n\n用户需求：{input}\n\n子任务："),
        });

        let resp = provider.chat(ChatRequest {
            model: self.model.clone(),
            messages: ctx,
            temperature: Some(0.3),
            max_tokens: Some(512),
            ..Default::default()
        }).await?;

        // 按行分割
        let subtasks: Vec<String> = resp.content
            .lines()
            .map(|l| l.trim().trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == '-' || c == ' '))
            .filter(|l| !l.is_empty())
            .map(|s| s.to_string())
            .collect();

        Ok(subtasks)
    }

    /// 派遣子Agent 执行任务
    pub async fn dispatch(
        &self,
        scheduler: Arc<Scheduler>,
        subtasks: Vec<String>,
    ) -> Vec<SubAgentResult> {
        let mut results = Vec::new();

        // 为每个子任务创建 Task 并入队
        for subtask in &subtasks {
            let task = Task {
                id: 0,
                scene_id: self.scene.unwrap_or(0),
                priority: Priority::Normal,
                kind: TaskKind::Reasoning,
                state: TaskState::Pending,
                retry_count: 0,
                max_retries: 3,
                timeout: std::time::Duration::from_secs(30),
                assigned_to: None,
                created_at: 0,
                tags: vec![],
                description: subtask.clone(),
            };
            scheduler.enqueue(task);
        }

        // 确保有足够的子Agent
        let needed = subtasks.len() as u64;
        let status = scheduler.status();
        for _ in status.active_subagents..needed as usize {
            scheduler.register_subagent();
        }

        // 分配并执行（简化版：逐个出队执行）
        while let Some(task) = scheduler.dequeue() {
            let provider = match &self.provider {
                Some(p) => p.clone(),
                None => break,
            };

            let result = match provider.chat(ChatRequest {
                model: self.model.clone(),
                messages: vec![Message {
                    role: Role::User,
                    content: task.description.clone(),
                }],
                temperature: Some(0.7),
                max_tokens: Some(4096),
                ..Default::default()
            }).await {
                Ok(resp) => resp.content,
                Err(e) => format!("✗ 执行失败: {e}"),
            };

            results.push(SubAgentResult {
                task_id: task.id,
                subtask: task.description,
                result,
                success: true,
            });
        }

        results
    }

    /// 合成子Agent 的结果为最终回复
    pub fn synthesize(&self, results: &[SubAgentResult]) -> String {
        if results.is_empty() {
            return "🧠 没有子任务被派发。".into();
        }
        let mut output = String::from("🧠 子Agent 执行结果：\n\n");
        for (i, r) in results.iter().enumerate() {
            output.push_str(&format!("**兵虫 #{}**\n", i + 1));
            output.push_str(&format!("  任务: {}\n", r.subtask));
            output.push_str(&format!("  {}\n\n", r.result));
        }
        output.push_str("── 星空虫族汇报完毕 ──");
        output
    }
}

impl Default for Overmind {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone)]
pub struct SubAgentResult {
    pub task_id: u64,
    pub subtask: String,
    pub result: String,
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overmind_default_is_main() {
        let om = Overmind::default();
        assert_eq!(om.identity.name, "Overmind");
        assert!(matches!(om.identity.kind, AgentKind::Main));
    }

    #[test]
    fn syntheize_empty_results() {
        let om = Overmind::default();
        let s = om.synthesize(&[]);
        assert!(s.contains("没有子任务"));
    }
}
