# Cleanup levels

After transcription, Verenu can run the raw text through an LLM cleanup step before pasting it. Cleanup intensity controls how much that step may change. Tone is separate and controls the voice, casing, punctuation style, and phrasing.

## The four levels

| Level | What it does |
| --- | --- |
| **Off** | Keeps the raw transcript. If Dual transcription is enabled, Verenu may still make a second call to reconcile two transcript candidates. |
| **Light** | Removes speech artifacts and fixes basic issues while keeping wording, order, and structure. |
| **Medium** (default) | Improves flow and removes redundancy while preserving each distinct detail. |
| **Strong** | Rewrites concisely while preserving facts, constraints, qualifiers, and emphasis. |

Use **Off** when you need the raw model output. **Light** is useful when wording and structure should remain close to what you said. **Medium** is the default for everyday dictation. **Strong** is for concise output when the important details still need to remain intact.

### Local Light cleanup

Local Light cleanup preserves the recognized words and their order. It handles a
limited set of clear spoken punctuation cues, scoped identifier underscores,
accidental article repetitions, and explicit weekday corrections before asking
the selected local model to adjust punctuation and capitalization. Literal text,
snippet expansions, technical tokens, and configured vocabulary are protected.
Ambiguous wording stays unchanged.

Model results and cached text must preserve that prepared transcript. An
unsupported edit, failed request, or truncated completion uses the prepared
transcript immediately, without another cleanup attempt or automatic escalation
to a different model. Medium and Strong retain their existing behavior.

English-specific rules require an explicitly English transcription language.
Local Light validates against the primary transcript; alternate-only wording is
not automatically accepted as a correction. Saved instructions can still apply
supported deterministic formatting, but requests for rewriting do not override
word preservation. Prompt audits measure the model response separately from the
pipeline's fallback protection.

## Changing it

- **During setup**: choose the default cleanup level in the first-run setup.
- **Later**: change the global setting in **Settings -> General**.
- **Per app or website**: configure a Context with its own cleanup intensity. See [Contexts](CONTEXTS.md).

## Next step

Explore [Contexts](CONTEXTS.md) to scope cleanup and tone, then see [Vocabulary](VOCABULARY.md) and [Snippets](SNIPPETS.md) for the content that belongs in each Context.

## Related docs

<p align="center">
  <a href="FIRST_DICTATION.md"><img alt="First Dictation" src="https://img.shields.io/badge/Back-First%20Dictation-7e7266"></a>
  <a href="CONTEXTS.md"><img alt="Contexts" src="https://img.shields.io/badge/Contexts-Guide-a3352b"></a>
</p>
