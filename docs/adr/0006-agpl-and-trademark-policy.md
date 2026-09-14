---
status: accepted
date: 2026-10-04
---

# AGPL-3.0 with a separate trademark policy

License the suite's original software under GNU AGPL version 3 (`AGPL-3.0-only`). Keep the standard license text unchanged and govern use of the Looski name and logo through a separate trademark policy. The goal is to preserve business owners' ability to use, modify, and obtain the source of the software they depend on, including covered modifications used to serve them over a network.

Commercial use, resale, paid hosting, and paid installation, customization, and support are permitted subject to the license. Distributing covered software requires the applicable notices, licensing, and corresponding-source compliance. A modified version used for remote network interaction must prominently offer its corresponding source to those users at no charge, as required by section 13. A compliant vendor may host the unmodified suite and charge customers.

AGPL does not require contributing changes upstream or publishing customer data and unrelated infrastructure. The precise boundary of a covered combined or derivative work depends on how components are integrated; do not assume every separate service or external API client becomes AGPL merely by communicating with the suite. Preserve upstream dependency licenses and validate compatibility before distribution.

The accepted trade-off is that compliant commercial competitors remain allowed. Strong copyleft preserves users' rights to covered software and modifications; it does not prohibit vendors from earning money. The separate trademark policy should address misleading claims of affiliation or official status while respecting lawful references to the software. Trademark rights must not be presented as a blanket ban on compliant resale or hosting.

The initial scaffold includes the standard AGPLv3 license file and source/upstream notices. Before publishing a release, provide the required corresponding-source access, complete dependency-license review, and finalize the trademark policy. This ADR records the decision; it does not itself finalize trademark terms.

## Decision history

The initial version of this ADR selected a source-available policy prohibiting resale and paid hosting. On October 4, 2026, that policy was replaced with AGPL-3.0 and separate trademark protection after accepting that the Open Source Definition requires commercial redistribution and use to remain permitted. The former no-resale/no-paid-hosting restrictions no longer apply to the planned license. Describe the intended releases as open source.

## References

- [OSI-approved GNU AGPLv3, including sections 4–7 and 13](https://opensource.org/license/agpl-3.0)
- [Open Source Definition](https://opensource.org/osd)
