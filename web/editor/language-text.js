// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: language words and checks: a language's name and what captions show of it by its code, the
   pickers' groups, the spoken languages Scribe hears, which languages are constructed or fictional and which the
   overlay writes in a script of their own, and the BCP 47 codes the server accepts
   (crates/sceneuron-core/src/subtitles/language.rs). */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const { CONSTRUCTED, HEARD, HEARD_NAMES, NATURAL, SCRIPTS, SPLIT } = inNode
    ? require("./languages.js")
    : window.SceneuronEditor.languages;

  /** The pickers' groups, each a label and its languages as [code, name]. */
  const GROUPS = [
    { label: "Languages", languages: NATURAL.map(([code, name]) => [code, name]) },
    { label: "Constructed and fictional", languages: CONSTRUCTED.map(([code, name]) => [code, name]) },
  ];

  /** The code for artificial languages as a group, which private-use tags for languages without a code start with. */
  const ARTIFICIAL = "art";

  function lower(code) {
    return String(code || "").trim().toLowerCase();
  }

  function find(list, wanted) {
    return list.find(([code]) => code.toLowerCase() === wanted);
  }

  /** The listed code that `code` is, without regard to case, such as "pt-BR" for "PT-br"; null when not listed. */
  function listedCode(code) {
    const found = find(NATURAL, lower(code)) || find(CONSTRUCTED, lower(code));
    return found ? found[0] : null;
  }

  /** Whether `code` names a constructed or fictional language: one listed, by code or language, or any under art. */
  function constructed(code) {
    const primary = lower(code).split("-")[0];
    return primary === ARTIFICIAL || Boolean(find(CONSTRUCTED, lower(code)) || find(CONSTRUCTED, primary));
  }

  /**
   * What is known of the language `code` names, by the whole code or else by what is left as its last subtag is
   * dropped, one at a time, so "es-MX" is Spanish, "sr-Latn-RS" Serbian (Latin), and "zh-TW" Chinese:
   * `{ code, name, captions, letter, constructed }`, or null when it is not listed.
   */
  function language(code) {
    const all = NATURAL.concat(CONSTRUCTED, SPLIT);
    const subtags = lower(code).split("-");
    for (let count = subtags.length; count > 0; count -= 1) {
      const found = find(all, subtags.slice(0, count).join("-"));
      if (found) {
        const [listed, name, captions, letter] = found;
        return { code: listed, name, captions, letter: letter || "", constructed: CONSTRUCTED.includes(found) };
      }
    }
    return null;
  }

  /** A language's name, such as "Quenya (Elvish)" for qya, or the code itself when it is not listed. */
  function languageName(code) {
    const known = language(code);
    return known ? known.name : String(code || "");
  }

  /** A language in words with its code, such as "Spanish (es)" or "Quenya (Elvish, qya)"; the code alone if unknown. */
  function languageLabel(code) {
    const text = String(code || "").trim();
    const known = language(text);
    if (!known) {
      return text;
    }
    return known.name.endsWith(")") ? `${known.name.slice(0, -1)}, ${text})` : `${known.name} (${text})`;
  }

  /** Whether ElevenLabs Scribe hears the spoken language `code`, as the spoken language field offers it. */
  function hears(code) {
    return HEARD.includes(lower(code));
  }

  /** The name of the script the overlay writes the language `code` in, such as "Tengwar" for qya; null for Latin. */
  function scriptName(code) {
    const primary = lower(code).split("-")[0];
    return Object.hasOwn(SCRIPTS, primary) ? SCRIPTS[primary] : null;
  }

  /** The spoken languages Scribe hears, as [code, name], in the order of their names. */
  function spokenLanguages() {
    const named = HEARD.map((code) => [code, HEARD_NAMES[code] || find(NATURAL, code)[1]]);
    return named.sort(([, one], [, other]) => one.localeCompare(other, "en"));
  }

  const MAX_TAG_CHARS = 35;
  // Where a subtag sits in a tag; RFC 5646 allows them only in this order.
  const [LANGUAGE, EXTENDED, SCRIPT, REGION, VARIANT, EXTENSION] = [0, 1, 2, 3, 4, 5];
  const ALPHANUMERIC = /^[A-Za-z0-9]{1,8}$/;
  const LETTERS = /^[A-Za-z]+$/;

  /** Where `subtag`, of 2 to 8 letters or digits after one at `after` and before any extension, sits; or null. */
  function nextPlace(subtag, after, extended) {
    const letters = LETTERS.test(subtag);
    if (subtag.length === 3 && letters && after <= EXTENDED && extended < 3) {
      return EXTENDED;
    }
    if (subtag.length === 4 && letters) {
      return after < SCRIPT ? SCRIPT : null;
    }
    if ((subtag.length === 2 && letters) || /^[0-9]{3}$/.test(subtag)) {
      return after < REGION ? REGION : null;
    }
    return subtag.length >= 5 || (subtag.length === 4 && /^[0-9]/.test(subtag)) ? VARIANT : null;
  }

  /**
   * Whether `tag` is a well-formed BCP 47 tag (RFC 5646) that starts with a language of 2 or 3 letters, as the server
   * checks it: then, each optional and in this order, up to three extended languages, a script, a region, variants,
   * extensions, and a private-use part, as in "art-x-dothraki". A variant or an extension's letter may not repeat.
   */
  function validTag(tag) {
    const text = String(tag);
    const subtags = text.split("-");
    if (text.length > MAX_TAG_CHARS || !/^[A-Za-z]{2,3}$/.test(subtags[0])) {
      return false;
    }
    let place = LANGUAGE;
    let extended = 0;
    // Variants and extension letters, neither of which may repeat.
    const seen = [];
    // The subtags after the last extension letter; it needs at least one.
    let extensionSubtags = 1;
    for (let index = 1; index < subtags.length; index += 1) {
      const subtag = subtags[index];
      const folded = subtag.toLowerCase();
      if (!ALPHANUMERIC.test(subtag)) {
        return false;
      }
      if (subtag.length === 1) {
        if (extensionSubtags === 0 || seen.includes(folded)) {
          return false;
        }
        if (folded === "x") {
          const rest = subtags.slice(index + 1);
          return rest.length > 0 && rest.every((part) => ALPHANUMERIC.test(part));
        }
        seen.push(folded);
        [place, extensionSubtags] = [EXTENSION, 0];
      } else if (place === EXTENSION) {
        extensionSubtags += 1;
      } else {
        const next = nextPlace(subtag, place, extended);
        if (next === null || (next === VARIANT && seen.includes(folded))) {
          return false;
        }
        place = next;
        extended += next === EXTENDED ? 1 : 0;
        if (next === VARIANT) {
          seen.push(folded);
        }
      }
    }
    return extensionSubtags > 0;
  }

  const languageText = {
    GROUPS,
    MAX_TAG_CHARS,
    constructed,
    hears,
    language,
    languageLabel,
    languageName,
    listedCode,
    scriptName,
    spokenLanguages,
    validTag,
  };
  if (inNode) {
    module.exports = languageText;
  } else {
    window.SceneuronEditor.languageText = languageText;
  }
})();
