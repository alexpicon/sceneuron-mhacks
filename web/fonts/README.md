# Fonts for scripts of their own

The overlay writes subtitles in Quenya and Sindarin in Tengwar, in Klingon in pIqaD, and in Toki
Pona in sitelen pona, at the code points the
[Under-ConScript Unicode Registry](https://www.kreativekorp.com/ucsur/) gives those scripts in the
Private Use Area. These fonts draw them. Each is the file its author published, unmodified, under
its own license, and `../script-fonts.css` uses each only for its script's block. The plugin
embeds them, and its `THIRD-PARTY-NOTICES.txt` carries their licenses, which `sceneuron-notices`
reads from this folder.

| File | Font | Covers | License | Source |
| --- | --- | --- | --- | --- |
| `AlcarinTengwarVF.woff2` | Alcarin Tengwar 0.83, variable weight 400 to 700, by Toshi Omagari | Tengwar, U+E000 to U+E07F, with mark positioning for the tehtar | SIL Open Font License 1.1, Reserved Font Name "Alcarin" ([`LICENSE-AlcarinTengwar.txt`](LICENSE-AlcarinTengwar.txt)) | [Tosche/Alcarin-Tengwar](https://github.com/Tosche/Alcarin-Tengwar), `Fonts Variable/` |
| `pIqaD-qolqoS.ttf` | pIqaD qolqoS 1.0 by Daniel Dadap | pIqaD, U+F8D0 to U+F8FF | SIL Open Font License 1.1, Reserved Font Name "pIqaD qolqoS" ([`LICENSE-pIqaD-qolqoS.txt`](LICENSE-pIqaD-qolqoS.txt), acknowledgements in [`NOTICE-pIqaD-qolqoS.txt`](NOTICE-pIqaD-qolqoS.txt)) | [dadap/pIqaD-fonts](https://github.com/dadap/pIqaD-fonts), release `pIqaD-fonts-r1` |
| `nasin-nanpa-4.0.2-UCSUR.otf` | nasin nanpa 4.0.2, its UCSUR-only build, by jan Itan | sitelen pona, U+F1900 to U+F19FF, with cartouches | MIT ([`LICENSE-nasin-nanpa.txt`](LICENSE-nasin-nanpa.txt)) | [ETBCOR/nasin-nanpa](https://github.com/ETBCOR/nasin-nanpa), `versions/` |

SHA-256 of each font as published:

```text
958d94acf184c8d286f5d04839c5adb2826837070e18a5f9e00566a760ad7789  AlcarinTengwarVF.woff2
af5c14da789b0e36e641fa326e454662fb7deb1cf1c72ec46451ad20e6860d84  pIqaD-qolqoS.ttf
f43918ef06c1db6205b0800083283c81800ba647d138acf3739021a582adea82  nasin-nanpa-4.0.2-UCSUR.otf
```

Alcarin Tengwar follows the Free Tengwar Font Project's mapping, which the Under-ConScript
registry's Tengwar block adopted in place of the original ConScript proposal. pIqaD qolqoS's glyph
at U+F8FF, the Klingon Empire logo, comes from another font whose author asks to be credited; the
notice credits them, and the overlay never uses the font for that code point. A font under the
Open Font License may not keep its reserved name if it is changed, so a subset or a conversion
would need a new name.

Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved. Proprietary software; see [LICENSE](../../LICENSE).
