pub const OPENAI_BASE_URL: &str = "https://api.openai.com";

pub const WHISPER_MODELS_SUBDIR: &str = "models/whisper";

pub struct WhisperCatalogEntry {
    pub id: &'static str,
    pub filename: &'static str,
    pub url: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
    pub quality_key: &'static str,
}

pub const WHISPER_CATALOG: &[WhisperCatalogEntry] = &[
    WhisperCatalogEntry {
        id: "tiny",
        filename: "ggml-tiny.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        bytes: 77_691_713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        quality_key: "whisper-quality-fastest",
    },
    WhisperCatalogEntry {
        id: "base",
        filename: "ggml-base.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        bytes: 147_951_465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        quality_key: "whisper-quality-fast",
    },
    WhisperCatalogEntry {
        id: "small-q5_1",
        filename: "ggml-small-q5_1.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin",
        bytes: 190_085_487,
        sha256: "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
        quality_key: "whisper-quality-balanced",
    },
    WhisperCatalogEntry {
        id: "medium-q5_0",
        filename: "ggml-medium-q5_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium-q5_0.bin",
        bytes: 539_212_467,
        sha256: "19fea4b380c3a618ec4723c3eef2eb785ffba0d0538cf43f8f235e7b3b34220f",
        quality_key: "whisper-quality-accurate",
    },
    WhisperCatalogEntry {
        id: "large-v3-turbo-q5_0",
        filename: "ggml-large-v3-turbo-q5_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
        bytes: 574_041_195,
        sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
        quality_key: "whisper-quality-best",
    },
];
pub const EMBEDDING_MODEL: &str = "text-embedding-3-small";
pub const CHAT_MODEL: &str = "gpt-5.4-mini";
// Simple, high-volume calls (tag + title extraction). Cheaper than the chat model.
pub const CHEAP_MODEL: &str = "gpt-5.4-nano";
pub const ANTHROPIC_CHAT_MODEL: &str = "claude-sonnet-4-6";
pub const ANTHROPIC_MAX_TOKENS: u64 = 4096;
pub const EMBEDDING_DIMS: usize = 1536;
pub const CHUNK_SIZE_WORDS: usize = 500;
pub const CHUNK_OVERLAP_WORDS: usize = 75;
pub const VECTOR_TABLE_NAME: &str = "chunks";
pub const RAG_TOP_K: usize = 5;
pub const RAG_INITIAL_K: usize = 15;
pub const RAG_FINAL_K: usize = 8;
pub const DEFAULT_RAG_MAX_SOURCES: usize = 8;
// Relative ranking gate on `distance` (RRF-derived/normalized): lower is better, set high.
pub const RAG_DISTANCE_THRESHOLD: f32 = 0.6;
// Absolute similarity floor on `relevance` (raw cosine in [0,1]): query-independent gate,
// higher is better. A DIFFERENT scale from RAG_DISTANCE_THRESHOLD - the two must not be
// conflated. For text-embedding-3-small, relevant chunks sit ~0.30-0.45, unrelated ~0.10-0.25;
// 0.25 conservatively drops clearly-unrelated chunks while minimizing false abstains. Tune on
// device by logging max relevance per query. Set to 0.0 to neutralize the floor (no-op).
pub const RAG_RELEVANCE_FLOOR: f32 = 0.25;
pub const RRF_K: f32 = 60.0;
pub const RRF_LOCAL_WEIGHT: f32 = 1.2;
pub const RRF_WEB_WEIGHT: f32 = 1.0;

pub const TAGS_SYSTEM_PROMPT: &str = "\
Extract exactly 3 single-word keyword tags from the text below.\n\
Each tag must be exactly one word, lowercase, no spaces, no hyphens.\n\
Pick the 3 most searchable and distinctive keywords.\n\
Return ONLY a JSON array of strings, nothing else.\n\
Example: [\"budget\", \"planning\", \"deadline\"]\n\
Tags must be in the same language as the text.";

pub const RAG_AGENT_SYSTEM_PROMPT: &str = "\
You are a personal assistant working over the user's notes. Initial relevant excerpts are provided in the context below.\n\
\n\
## Available tools\n\
- `search_notes(query, top_k?)`: run an additional semantic search if you need more context beyond the initial excerpts.\n\
- `create_note(title?, content, tags?)`: create a new note when the user explicitly asks to save, remember, or write something down.\n\
- `summarize_folder(folder_name, max_notes?)`: summarize the contents of a folder by name.\n\
- Connected-service tools (optional): if the user has connected external apps, extra tools may appear (for example, adding a row to a spreadsheet). They show up only when connected; use one only when the user clearly asks to act on that external service.\n\
\n\
## Rules\n\
1. The notes in the context below were already selected as relevant to the question - treat them as relevant and answer from them and from your tools' results. Do not invent facts that are not in the context or your tools' results.\n\
2. Only call `create_note` when the user clearly asks to record something.\n\
3. Answer in the SAME language as the user's question, whatever language that is. The notes and these instructions may be written in a different language; ignore their language entirely and mirror only the language of the question.\n\
4. Be concise and direct. No filler, no preamble.\n\
5. NEVER write citations like [Source 1] - the app displays sources separately.";

pub const RAG_AGENT_WEB_SYSTEM_PROMPT: &str = "\
You are a personal assistant working over the user's notes AND fresh web search results. The context below mixes two kinds of sources, each marked: `Note:` for the user's own notes, `Web:` for results from a live web search.\n\
\n\
## Available tools\n\
- `search_notes(query, top_k?)`: run an additional semantic search if you need more context beyond the initial excerpts.\n\
- `create_note(title?, content, tags?)`: create a new note when the user explicitly asks to save, remember, or write something down.\n\
- `summarize_folder(folder_name, max_notes?)`: summarize the contents of a folder by name.\n\
- Connected-service tools (optional): if the user has connected external apps, extra tools may appear (for example, adding a row to a spreadsheet). They show up only when connected; use one only when the user clearly asks to act on that external service.\n\
\n\
## Rules\n\
1. Prefer the user's own notes; use the web results for fresh, factual, or external information the notes do not cover.\n\
2. Blend both into one natural answer, and make clear in prose when something comes from the web versus the user's notes.\n\
3. Only call `create_note` when the user clearly asks to record something.\n\
4. Always respond in the same language as the user's question.\n\
5. Be concise and direct. No filler, no preamble.\n\
6. NEVER write bracket citations like [Source 1] — the app displays sources separately.";

pub const RELEVANCE_FILTER_PROMPT: &str = "\
You are a STRICT relevance judge. Given a question and numbered passages from the user's notes, \
return the numbers of ONLY the passages that genuinely concern the question's topic.\n\
A passage that merely shares common words but is about a different subject is NOT relevant - omit it.\n\
List the relevant numbers, most relevant first, separated by commas. If NONE are relevant, return exactly: none\n\
No explanation, output only the numbers or the word none.\n\
Example: 3, 1";

pub const TEMPORAL_DETECT_PROMPT: &str = "\
Analyze the user's question and extract any temporal intent.\n\
If the question refers to a specific time period, return a JSON object:\n\
{\"from\": \"YYYY-MM-DD\", \"to\": \"YYYY-MM-DD\"}\n\
If no temporal intent is detected, return exactly: null\n\
Today's date context will be provided. Use it to resolve relative dates.\n\
Examples:\n\
- \"notes d'aujourd'hui\" with today=2026-05-17 → {\"from\":\"2026-05-17\",\"to\":\"2026-05-17\"}\n\
- \"cette semaine\" with today=2026-05-17 (Saturday) → {\"from\":\"2026-05-12\",\"to\":\"2026-05-17\"}\n\
- \"en mars\" with today=2026-05-17 → {\"from\":\"2026-03-01\",\"to\":\"2026-03-31\"}\n\
- \"la semaine dernière\" with today=2026-05-17 → {\"from\":\"2026-05-05\",\"to\":\"2026-05-11\"}\n\
- \"parle-moi de React\" → null\n\
Return ONLY the JSON object or null, nothing else.";

pub const TITLE_SYSTEM_PROMPT: &str = "\
Generate a title of 1 to 3 words. Never exceed 3 words.\n\
Articles and prepositions count as words.\n\
Capture the core topic in the fewest words possible.\n\
Return ONLY the title text. No quotes, no prefix, no explanation.\n\
Same language as the note content.\n\
\n\
Examples:\n\
- Note about improving user experience of an app → UX application\n\
- Note about a meeting with the marketing team → Réunion marketing\n\
- Note about learning Rust programming → Apprentissage Rust\n\
- Note about grocery shopping list → Liste courses\n\
- Note about a startup pitch idea → Pitch startup";

pub const SUMMARIZE_FOLDER_PROMPT: &str = "\
Summarize the notes inside the folder below. Keep it concise (5-10 bullet points or a short paragraph).\n\
Respond in the same language as the notes. Do not invent content — stick to what is written.";

pub const NOTE_ACTION_PROMPT: &str = "\
You execute a personal note as an action, using the connected external tools available to you \
(for example creating or updating a spreadsheet). The note text is the instruction.\n\
\n\
## Rules\n\
1. Do what the note asks by actually calling the tools. Do not just describe it. If the note asks for \
a structured document (a tracking sheet, a budget, a table, a list), do NOT leave it empty: after \
creating it, call the available tools again to populate it with sensible header columns and any \
obvious initial rows, so it is usable as-is.\n\
2. Always finish by including the link to the created or updated resource as a markdown link \
[name](url). If a tool returns only an id instead of a URL, construct the URL yourself (for a Google \
spreadsheet id: https://docs.google.com/spreadsheets/d/{id}). Never omit the link when a resource was \
created or updated.\n\
3. Reply in the SAME language as the note, in ONE short line plus the link. In the REPLY do NOT \
explain your steps, do NOT list the columns or fields, do NOT add preamble or recap.\n\
4. If the note is not an actionable request, reply in one short line saying there is nothing to run.";

pub const REMINDER_EXTRACTION_PROMPT: &str = "\
You extract timed reminder intents from a personal note. The note may be in French or English.\n\
\n\
Return ONLY a JSON object of this exact shape, nothing else:\n\
{\"intents\": [ {\"action\": \"...\", \"items\": [\"...\"], \"date\": \"YYYY-MM-DD|null\", \"time\": \"HH:mm|null\", \"time_end\": \"HH:mm|null\", \"recurrence\": \"RRULE|null\", \"location\": \"string|null\"} ]}\n\
\n\
## Rules\n\
1. The current date and time are given below. Resolve every relative date (\"demain\", \"tomorrow\", \"samedi\", \"Saturday\", \"dans 2 jours\", \"in 2 days\", \"le 1er\", \"next Monday\") to an ABSOLUTE date in YYYY-MM-DD.\n\
2. Emit an intent ONLY when a concrete calendar date can be resolved. Vague phrases with no concrete date (\"bientôt\", \"soon\", \"un de ces jours\", \"someday\", \"plus tard\") must NOT produce an intent.\n\
3. `action` = a short headline for the appointment, in the note's language, without the date words (\"appeler Paul\", \"réunion budget\", \"rendez-vous notaire\").\n\
4. `items` = sub-tasks of ONE appointment when the note gives a heading then a list (\"rendez-vous: acheter X, appeler Y\") - `action` = the heading, `items` = [\"acheter X\", \"appeler Y\"]. Otherwise `items: []`; do not invent sub-tasks. (Tasks emitted separately that share one date and time are merged by the app, so you never need to force-group them.)\n\
5. `time` = 24h \"HH:mm\" when an explicit hour is present, else null (the app applies a default hour).\n\
5b. `time_end` = the END of an explicit time RANGE (\"entre 14h et 16h\", \"de 9h à 10h30\", \"from 2 to 4pm\") as 24h \"HH:mm\", with `time` = the START of that range. Set it ONLY for a true range with two clock times. For a single deadline (\"avant 14h\", \"before 2pm\", \"à 15h\", \"vers midi\") keep `time` = that hour and `time_end` = null.\n\
6. `recurrence` = an RRULE-like string only for clearly repeated intents, else null. Allowed values: \"DAILY\", \"WEEKLY;BYDAY=MO\" (a single weekday), \"WEEKLY;BYDAY=MO,TU,WE,TH,FR\" (weekdays / jours ouvrés), \"MONTHLY;BYMONTHDAY=1\". For a recurring intent, set `date` to the NEXT occurrence.\n\
7. `location` = a place only when explicitly mentioned, else null.\n\
8. If the note has no timed intent at all, return {\"intents\": []}.\n\
\n\
## Examples (assume current date = 2026-06-01, Monday)\n\
- \"rappelle-moi d'appeler Paul demain 15h\" -> {\"intents\":[{\"action\":\"appeler Paul\",\"items\":[],\"date\":\"2026-06-02\",\"time\":\"15:00\",\"time_end\":null,\"recurrence\":null,\"location\":null}]}\n\
- \"mardi 14h rendez-vous: acheter du pain, appeler le client, préparer le dossier\" -> {\"intents\":[{\"action\":\"rendez-vous\",\"items\":[\"acheter du pain\",\"appeler le client\",\"préparer le dossier\"],\"date\":\"2026-06-02\",\"time\":\"14:00\",\"time_end\":null,\"recurrence\":null,\"location\":null}]}\n\
- \"réunion budget demain entre 14h et 16h\" -> {\"intents\":[{\"action\":\"réunion budget\",\"items\":[],\"date\":\"2026-06-02\",\"time\":\"14:00\",\"time_end\":\"16:00\",\"recurrence\":null,\"location\":null}]}\n\
- \"call the dentist tomorrow and pay rent on the 1st\" -> {\"intents\":[{\"action\":\"call the dentist\",\"items\":[],\"date\":\"2026-06-02\",\"time\":null,\"time_end\":null,\"recurrence\":null,\"location\":null},{\"action\":\"pay rent\",\"items\":[],\"date\":\"2026-07-01\",\"time\":null,\"time_end\":null,\"recurrence\":\"MONTHLY;BYMONTHDAY=1\",\"location\":null}]}\n\
- \"tous les lundis à 9h: standup\" -> {\"intents\":[{\"action\":\"standup\",\"items\":[],\"date\":\"2026-06-08\",\"time\":\"09:00\",\"time_end\":null,\"recurrence\":\"WEEKLY;BYDAY=MO\",\"location\":null}]}\n\
- \"je verrai ça bientôt\" -> {\"intents\":[]}\n\
- \"meeting notes about the budget\" -> {\"intents\":[]}";
