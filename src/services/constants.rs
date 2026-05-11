pub const OPENAI_BASE_URL: &str = "https://api.openai.com";
pub const EMBEDDING_MODEL: &str = "text-embedding-3-small";
pub const CHAT_MODEL: &str = "gpt-4o-mini";
pub const EMBEDDING_DIMS: usize = 1536;
pub const CHUNK_SIZE_WORDS: usize = 375;
pub const CHUNK_OVERLAP_WORDS: usize = 37;
pub const VECTOR_TABLE_NAME: &str = "chunks";
pub const RAG_TOP_K: usize = 5;

pub const TAGS_SYSTEM_PROMPT: &str = "\
Extract 3 to 5 short tags (1-3 words each) from the text below.\n\
Return ONLY a JSON array of strings, nothing else.\n\
Example: [\"meeting notes\", \"project alpha\", \"deadline\"]\n\
Tags must be in the same language as the text.";

pub const RAG_SYSTEM_PROMPT: &str = "\
You are a personal assistant that answers questions based on the user's notes provided below as context.\n\
\n\
## Rules\n\
1. Use ONLY information from the provided context. Never invent or use external knowledge.\n\
2. If multiple notes are relevant, synthesize them naturally.\n\
3. Always respond in the same language as the user's question.\n\
4. Be concise and direct. No filler, no preamble.\n\
5. For broad questions (\"what's in my notes?\", \"summarize\"), give an overview of ALL the provided notes.\n\
6. For specific questions, focus on the most relevant notes.\n\
\n\
## Response format\n\
- Answer the question directly in flowing prose.\n\
- NEVER write citations like [Source 1], [Note title], or any bracketed references.\n\
- NEVER list sources at the end — the app displays them separately.\n\
- Just answer naturally without any source markup.";

pub const RAG_AGENT_SYSTEM_PROMPT: &str = "\
You are a personal assistant working over the user's notes. Initial relevant excerpts are provided in the context below.\n\
\n\
## Available tools\n\
- `search_notes(query, top_k?)`: run an additional semantic search if you need more context beyond the initial excerpts.\n\
- `create_note(title?, content, tags?)`: create a new note when the user explicitly asks to save, remember, or write something down.\n\
- `summarize_folder(folder_name, max_notes?)`: summarize the contents of a folder by name.\n\
\n\
## Rules\n\
1. Prefer the provided context first; only call `search_notes` if the question clearly needs more notes.\n\
2. Only call `create_note` when the user clearly asks to record something.\n\
3. Always respond in the same language as the user's question.\n\
4. Be concise and direct. No filler, no preamble.\n\
5. NEVER write citations like [Source 1] — the app displays sources separately.";

pub const SUMMARIZE_FOLDER_PROMPT: &str = "\
Summarize the notes inside the folder below. Keep it concise (5-10 bullet points or a short paragraph).\n\
Respond in the same language as the notes. Do not invent content — stick to what is written.";
