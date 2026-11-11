# 08 — AI and LLM Integration

Lekhani’s AI support is narrow: a single blocking chat-completions request that returns either corrected LaTeX or a TeX package name. It is not a chat assistant, does not stream, and does not retain history.

## 1. Configuration model

Two modes, selected by `llm_settings_tab_index`:

| Index | UI label | Required fields | Endpoint used for requests |
|---|---|---|---|
| `0` | Local LLM | model + endpoint URL | the stored `llm_endpoint_url`, verbatim |
| `1` | API Key | model + API key | **hard-coded** `https://api.openai.com/v1/chat/completions` |

Important: in local mode the stored endpoint is used **as-is**. It is not normalized and `/chat/completions` is not appended. The default `http://localhost:1234/v1` is therefore a base URL that will return 404 unless the user enters the full completions URL (e.g. `http://localhost:1234/v1/chat/completions`). In API mode, the stored endpoint is ignored entirely.

### Persisted fields

`dirs::config_dir()/lekhani-latex/config.json`:

```json
{
  "llm_api_key": "...",
  "llm_settings_tab_index": 0,
  "llm_configured": true,
  "llm_model": "llama-3.2-3b",
  "llm_custom_model": "",
  "llm_endpoint_url": "http://localhost:1234/v1",
  "theme": "System"
}
```

- `llm_configured` is a user-level gate, not a connectivity check.
- The API key is stored in plaintext.
- `LLM_API_KEY` is read only when no stored key exists; it is not persisted.
- The settings dialog writes on Save, Cancel, or window close.
- Deleting a configuration sets `llm_configured = false`, clears the key, and resets the endpoint to the default for the current mode.

### Presets

`LLM_PRESETS` in `gui/src/app/mod.rs`:

| Display name | Model ID |
|---|---|
| Llama 3.2 3B | `llama-3.2-3b` |
| Llama 3.1 8B | `llama-3.1-8b` |
| Mistral 7B | `mistral-7b` |
| Gemma 2 9B | `gemma-2-9b` |
| Phi-3 Mini | `phi-3-mini` |
| Qwen 2.5 7B | `qwen-2.5-7b` |
| Qwen 2.5 14B | `qwen-2.5-14b` |
| DeepSeek-V2 | `deepseek-v2` |

A “Custom” entry allows any string. `init_llm_preset_from_model` reconciles a loaded model with the preset list; if it does not match a preset, it treats the model as custom.

## 2. Where AI is triggered

| Entry point | Scope | Code |
|---|---|---|
| Toolbar “✨ Fix with AI” | whole document | `trigger_llm_correction()` → `do_llm_correction(None)` |
| Editor context menu “✨ Fix with AI” | hovered line | `trigger_llm_correction_for_line(line)` → `do_llm_correction(Some(line))` |

Gating:

- The toolbar button is enabled only when a tab exists, an LLM is configured, no correction is in flight, and `active_tab.error_message.is_some()`.
- The context-menu item appears only when configured and is enabled only when there is an error.
- The flag is global: only one request can run at a time, across all tabs.

## 3. Request construction

`do_llm_correction` runs on the UI thread to build the prompt, then spawns a `std::thread` for the HTTP call.

### Data sent

- `errors = tab.error_message.clone().unwrap_or_default()` — the joined compiler error text from the last failure.
- `text = tab.buffer.text.clone()`, or the target line if `target_line` is set.
- `model = llm_custom_model` if non-empty, else `llm_model`; if that is empty, `"gpt-4o-mini"`.
- `api_key` as `Authorization: Bearer <key>`.
- Endpoint as described above.
- Body: a minimal OpenAI-compatible object:

```json
{
  "model": "<model>",
  "messages": [{"role": "user", "content": "<prompt>"}]
}
```

No `temperature`, `max_tokens`, `stream`, or system message is sent.

### Prompt contract

Line mode:

```text
Fix any LaTeX syntax errors in the following line based on these errors: <errors>

Line:
<line>

Return a single JSON object (NOT an array): if the error EXPLICITLY states a missing .sty file, use {"action":"install","package":"name","explanation":"short explanation"}. Otherwise, if correcting code, use {"action":"correct","text":"corrected line","explanation":"short explanation"}. Do not guess missing packages for undefined control sequences; fix the code instead. If correcting, provide the FULL corrected line WITHOUT adding any new comments to the code. Keep the explanation short to save tokens. Return ONLY the JSON.
```

Document mode is equivalent but asks for the entire document and explicitly requires all corrections to be applied.

The prompt is intentionally token-economical. If changing it, preserve the “one JSON object” requirement, the `install` vs `correct` distinction, and the “no guessed packages” rule; the response parser depends on the shape.

## 4. Response parsing

The thread reads the whole response body with `ureq`, parses it as JSON, and takes `choices[0].message.content`. Then:

1. If the content starts with ```` ```json ````, strip that fence and a trailing ```` ``` ````.
2. Parse the remaining string as JSON.
3. If the result is an array, use its first element.
4. Read `explanation` (optional).
5. Dispatch on `action`:
   - `"install"` + non-empty `package` → `LlmAction::InstallPackage`.
   - `"correct"` + non-empty `text` → `LlmAction::Correction`.
6. Anything else → `Err("Invalid response format from LLM. Raw response: …")`.

Other error strings:

- `"No content in LLM response"` — `choices[0].message.content` missing.
- `"Failed to parse LLM response"` — outer JSON invalid.
- `"LLM network error: <e>"` — request failure.

All are shown as a red line in the AI output tab and copied into `error_message`.

## 5. Applying a correction

`App::poll_events` drains the LLM receiver.

### Whole-document correction

`tab.buffer.replace_all(&text)`:

- pushes a delete token for the old text and an insert token for the new text;
- replaces the buffer, sets the cursor to the end, marks dirty.

The undo stack therefore contains a single “replace everything” step (two tokens).

### Line correction

- `lines = tab.buffer.text.lines().collect()`.
- If `line` is 1-based and within range, rebuild the text line by line, substituting the corrected line.
- Preserve the original lack of a trailing newline if the old text did not end with one.
- Call `replace_all(&new_text)`.

If the requested line is out of range, the buffer is left unchanged but the explanation is still logged and the status says “Syntax corrected via LLM”.

### Package install

- Logs “Attempting to install package <pkg>…” (yellow).
- Sets status “Installing package <pkg>…”.
- Spawns a thread running `tlmgr install <pkg>` (no shell, no elevation).
- Sends `UpdateMessage::PackageInstalled(pkg, result)` to the shared update channel.
- Success/failure is appended to the AI log in green/red; the app does **not** automatically recompile after a successful install.

`tlmgr` must be on `PATH` and the user must have write permission to the TeX tree. On a system TeX Live installation this usually requires root; the command will fail with a permission error, which is shown verbatim.

## 6. Output and state

- `tab.ai_output_log` is cleared at the start of each correction.
- On success, the explanation is green; the generic “Correction applied successfully.” is used when the model omitted one.
- `output_panel_tab` switches to `AI` and the panel opens after any result or error.
- `llm_correction_in_progress` is cleared as soon as any result is received, including errors.
- There is no cancellation. Closing the tab during a request means the result is applied to whichever tab is active when it arrives (the handler uses `active_tab_mut()`).

## 7. Security and privacy notes

- The entire document or hovered line plus the compiler error text is sent to the configured endpoint.
- The API key is stored unencrypted and sent as a bearer token.
- Local endpoints receive the key too, even if the server ignores it.
- There is no TLS pinning, proxy configuration, timeout, or retry beyond `ureq` defaults.
- The prompt asks the model to return code; the code is inserted verbatim. There is no validation or sandboxing, but the app only writes the text into the buffer — it does not execute model output.
- `tlmgr install` is executed with the package string supplied by the model. The command is spawned directly (no shell), so shell metacharacters are not interpreted, but a malicious package name can still install an arbitrary TeX package from the configured repository.

## 8. Known limitations and bugs

| Limitation | Detail |
|---|---|
| Placeholder update URLs | Update checking is broken, but unrelated to LLM. |
| Local endpoint shape | The default base URL lacks `/chat/completions`. |
| API endpoint ignored | In API mode, custom endpoint URLs are never used. |
| No connectivity test | Save does not verify the key/endpoint. |
| No streaming | The UI shows “Fixing…” until the full response arrives. |
| Global single request | A request is tied to the active tab only at application time. |
| Settings save on Cancel | Cancel/close persists edited values. |
| Configured fields locked | Editing requires Delete first. |
| No auto-recompile | A successful package install does not retry compilation. |
| No history | Each request is independent. |
| Model output can be an array | Parser accepts it despite the prompt asking for an object. |
