use async_openai::Client;
use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessageArgs,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestToolMessageArgs, ChatCompletionRequestUserMessageArgs, ChatCompletionTool,
    ChatCompletionTools, CreateChatCompletionRequestArgs, FunctionObjectArgs,
};
use serde_json::Value;

pub type Message = ChatCompletionRequestMessage;

pub fn system_message(content: &str) -> Message {
    ChatCompletionRequestSystemMessageArgs::default()
        .content(content)
        .build()
        .unwrap()
        .into()
}

pub fn initial_history(system_prompt: &str) -> Vec<Message> {
    vec![system_message(system_prompt)]
}

// ---------------------------------------------------------------------------
// OpenAi — wraps async-openai with a blocking interface
// ---------------------------------------------------------------------------

pub struct OpenAi {
    client: Client<async_openai::config::OpenAIConfig>,
    model: String,
}

impl OpenAi {
    /// Create a new instance.  Reads `OPENAI_API_KEY` from the environment.
    pub fn new(model: &str) -> Self {
        let client = Client::new();
        Self {
            client,
            model: model.into(),
        }
    }

    /// Send a user query and return the assistant's text reply (blocking).
    /// Tool calls are dispatched via `execute_tool(name, args) -> result`.
    pub fn ask(
        &self,
        query: &str,
        history: &mut Vec<Message>,
        tools_json: &[Value],
        execute_tool: &mut impl FnMut(&str, Value) -> String,
    ) -> String {
        let user_msg: Message = ChatCompletionRequestUserMessageArgs::default()
            .content(query)
            .build()
            .unwrap()
            .into();
        history.push(user_msg);

        let tools = Self::convert_tools(tools_json);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async { self.ask_loop(history, &tools, execute_tool).await })
    }

    /// Async variant for use inside an existing tokio runtime (e.g. Discord bot).
    pub async fn ask_async(
        &self,
        query: &str,
        history: &mut Vec<Message>,
        tools_json: &[Value],
        execute_tool: &mut impl FnMut(&str, Value) -> String,
    ) -> String {
        let user_msg: Message = ChatCompletionRequestUserMessageArgs::default()
            .content(query)
            .build()
            .unwrap()
            .into();
        history.push(user_msg);

        let tools = Self::convert_tools(tools_json);
        self.ask_loop(history, &tools, execute_tool).await
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    async fn ask_loop(
        &self,
        history: &mut Vec<Message>,
        tools: &[ChatCompletionTools],
        execute_tool: &mut impl FnMut(&str, Value) -> String,
    ) -> String {
        loop {
            let mut req = CreateChatCompletionRequestArgs::default();
            req.model(&self.model).messages(history.clone());
            if !tools.is_empty() {
                req.tools(tools.to_vec());
            }

            let request = match req.build() {
                Ok(r) => r,
                Err(e) => return format!("Ошибка: {}", e),
            };

            let response = match self.client.chat().create(request).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("OpenAI error: {}", e);
                    return format!("Ошибка OpenAI: {}", e);
                }
            };

            let choice = &response.choices[0];

            // ---- handle tool calls ----
            if let Some(ref tool_calls) = choice.message.tool_calls {
                if !tool_calls.is_empty() {
                    let asst: Message = ChatCompletionRequestAssistantMessageArgs::default()
                        .tool_calls(tool_calls.clone())
                        .build()
                        .unwrap()
                        .into();
                    history.push(asst);

                    for tc_variant in tool_calls {
                        let ChatCompletionMessageToolCalls::Function(tc) = tc_variant else {
                            continue;
                        };

                        let args: Value =
                            serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                        let result = execute_tool(&tc.function.name, args);

                        let tool_msg: Message = ChatCompletionRequestToolMessageArgs::default()
                            .tool_call_id(&tc.id)
                            .content(result)
                            .build()
                            .unwrap()
                            .into();
                        history.push(tool_msg);
                    }
                    continue;
                }
            }

            // ---- plain text response ----
            let content = choice.message.content.clone().unwrap_or_default();
            let asst: Message = ChatCompletionRequestAssistantMessageArgs::default()
                .content(content.clone())
                .build()
                .unwrap()
                .into();
            history.push(asst);
            return content;
        }
    }

    /// Convert raw tool definitions (name, description, inputSchema) to
    /// the async-openai typed representation.
    fn convert_tools(raw_tools: &[Value]) -> Vec<ChatCompletionTools> {
        raw_tools
            .iter()
            .filter_map(|t| {
                let name = t["name"].as_str()?;
                let desc = t["description"].as_str().unwrap_or("");
                let params = t["inputSchema"].clone();

                let func = FunctionObjectArgs::default()
                    .name(name)
                    .description(desc)
                    .parameters(params)
                    .build()
                    .ok()?;

                Some(ChatCompletionTools::Function(ChatCompletionTool {
                    function: func,
                }))
            })
            .collect()
    }
}
