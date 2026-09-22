# SaaS: origins, scale, and the case for software price compression

Research date: 12 October 2026. Purpose: source material for the Looski video series, not an investment forecast. All dollar amounts are USD. Supporting fact tables: [history](saas-history-sources.md) and [market size](saas-market-sources.md).

## Central finding

The evidence supports a threat to the pricing power of reproducible application software. It does not establish that total SaaS revenue or company value will reach zero. A defensible thesis is: **as creating and adapting ordinary business software becomes cheaper, charging recurring premiums merely for access to familiar features becomes harder to sustain.** The direction, magnitude and timing remain predictions.

Use “pure SaaS” in the project-specific sense defined in [intent.md](../../intent.md): products whose principal deliverable is application software, such as CRM, forms, CMS and project management. This excludes the project's named infrastructure, payment, carrier, proprietary-data and other categories. It is not an analyst market segment with a separately measured global valuation. The broad market figures below provide context, not the value of everything Looski can replace.

## 1. The beginning

Modern SaaS emerged in the late 1990s, building on earlier hosted computing. NetSuite was founded in 1998 and released NetLedger in 1999; Salesforce was founded in 1999 and began its service in February 2000. See the contemporaneous filings in the [history source table](saas-history-sources.md#defensible-historical-framing). Avoid an unsupported first-inventor claim.

The business proposition was practical: let a provider operate the application and deliver it online. The buyer could start without deploying the entire stack. NIST's definition includes access through APIs as well as thin clients, so an agent using a hosted API can still be consuming SaaS. [NIST SP 800-145](https://nvlpubs.nist.gov/nistpubs/legacy/sp/nistspecialpublication800-145.pdf).

## 2. Why it grew

Historical synthesis: better internet access enabled delivery; shared infrastructure spread costs; subscriptions lowered the initial purchasing hurdle; vendors could expand accounts through additional users and products. These mechanisms explain the appeal, rather than proving a single cause of all industry growth. Contemporaneous company explanations and adoption evidence are recorded in the [growth notes](saas-history-sources.md#why-adoption-could-accelerate).

For a consistent chart, use Salesforce's annual reported revenue: **$96 million in FY2004, $1.08 billion in FY2009, $4.07 billion in FY2014, $13.28 billion in FY2019, and $41.53 billion in FY2026**. The [chart source table](saas-history-sources.md#a-chart-that-demonstrates-the-growth) links each filing. This is one company's growth, includes acquisitions and changing products, and uses fiscal years ending January 31. Do not label it global SaaS revenue or organic growth.

Editorial interpretation: the same recurring relationship that removed operational work also made the provider central to customers' workflows. Whether that dependence is valuable convenience or unwanted lock-in depends on the product, portability and customer.

## 3. Current rough scale

| Metric | Usable figure | Meaning |
|---|---|---|
| Annual SaaS spending | **More than $500 billion implied for 2026** | IDC forecasts public cloud spending above $1 trillion and SaaS above half. This is an inference from a forecast, not completed annual revenue. [IDC, March 2026](https://www.idc.com/resource-center/press-releases/publiccloudspend2026/). |
| Listed cloud company equity value | **Approximately $2.8 trillion** | BVP's selected public cloud basket, as displayed when checked October 12. It is not the entire SaaS industry or a valuation of Looski's addressable segment. [BVP dashboard](https://cloudindex.bvp.com/). |

Gartner's older November 2024 forecast placed 2025 SaaS spending at $299.1 billion. Do not splice that number and IDC's estimate into a growth chart: periods, definitions and forecast vintages differ. Full definitions and links are in the [market notes](saas-market-sources.md).

## 4. The strongest version of “going to zero”

The following is an economic argument to test, not an observed industry endpoint.

**Cheaper implementation weakens feature scarcity.** A new entrant need not reproduce every capability of a large vendor to replace the subset a particular business uses. If many builders can cheaply supply that subset, customers gain alternatives and bargaining power. Anthropic's June 2026 analysis of approximately 400,000 sessions finds substantial execution delegated to coding agents, while human domain expertise remains associated with success. This is observational vendor research, not a measured percentage reduction in complete-product costs. [Anthropic research](https://www.anthropic.com/research/claude-code-expertise).

**Cheaper model use can help make customization affordable.** Stanford documented a greater-than-280-fold drop in inference price at a fixed GPT-3.5-level benchmark between November 2022 and October 2024. That is a historical benchmark-price comparison, not a 280-fold reduction in software engineering costs or proof inference is free. [Stanford AI Index 2025](https://hai.stanford.edu/ai-index/2025-ai-index-report/research-and-development).

**Agents may weaken the relationship between employees and paid seats.** When an agent executes a workflow across systems, fewer people may need each application's interface. Gartner forecasts up to $234 billion of enterprise application spending exposed to agentic arbitrage through 2030. Exposure is not realized loss. [Gartner, July 2026](https://www.gartner.com/en/newsroom/press-releases/2026-07-01-gartner-says-us-dollars-234-billion-in-enterprise-application-software-spend-is-at-risk-from-agentic-artificial-intelligence).

**Open distribution changes who needs to recover the development cost.** Looski's proposed model spreads a common foundation across modules and users, with no software license fee for using the published open-source code. Businesses still pay for hardware, operations, outside services and possibly expert assistance. Success requires a sustainable way to maintain the shared code. This is the project's proposed mechanism, not a proven outcome.

**Portability is part of the mechanism.** Lower coding cost alone cannot free customers from inaccessible data, restricted APIs, difficult migrations or embedded business processes. An agent-friendly replacement needs usable exports, documented contracts, permissions and safe migration. Otherwise the incumbent may retain the customer relationship despite abundant alternative code.

## 5. What does not follow

- **An inexpensive prototype does not establish inexpensive lifetime ownership.** Reliability, migrations, recovery, security fixes and operational support remain work. METR's February 2026 update says its newer productivity estimates are unreliable because of selection and measurement problems. It expects better speedups than in early 2025, but does not establish a universal multiplier. [METR](https://metr.org/blog/2026-02-24-uplift-update/).
- **Losing seats need not mean losing all revenue.** Vendors can charge for usage or outcomes. Intercom currently lists $0.99 for a qualifying Fin resolution over chat/email with Intercom, alongside other outcome categories and pricing. This illustrates a changed charging unit, not disappearance of paid software. [Intercom pricing definitions](https://www.intercom.com/help/en/articles/8205718-fin-ai-agent-outcomes).
- **Incumbents can use the same technology.** Microsoft announced MCP servers for Dynamics 365 ERP and CRM in May 2025. Agents can make existing systems more useful rather than replace them. [Microsoft announcement](https://www.microsoft.com/en-us/dynamics-365/blog/business-leader/2025/05/20/the-autonomous-enterprise-how-generative-ai-is-reshaping-business-applications/).
- **Current financial results do not show universal collapse.** Salesforce reported $11.3 billion revenue for its quarter ended July 31, 2026, up 11%, including acquisition revenue. [Company results](https://investor.salesforce.com/news/news-details/2026/Salesforce-Delivers-Record-Second-Quarter-Fiscal-2027-Results/default.aspx).
- **Abundant code does not guarantee competitive markets.** Distribution, reputation, proprietary data and switching costs can preserve pricing power. Lower prices may also encourage more software use, leaving aggregate spending higher. These are countermechanisms to the thesis, not quantified forecasts here.

The exact zero claim should therefore concern the **license fee for selected, freely distributed application functionality**. The broader claim is downward pressure on recurring software premiums. Zero total cost, zero business value and zero industry market capitalization are substantially different claims without support in this research.

## 6. Suggested first-video argument

Original editorial wording, not a quotation from a source:

> SaaS won because owning business software was a headache. Someone else ran the servers, shipped the updates, and kept it working. That was worth paying for.
>
> My bet is that AI changes what we need to keep renting. If the everyday software a business needs becomes cheap to build, adapt, and share, how much should we keep paying just to access those features?
>
> I'm going to test that by building open-source replacements. I'll show the workflows they replace, the costs that remain, and what breaks. The test is whether a business can actually cancel a subscription and keep doing its work.

Suggested sequence: origins and real customer benefit → evidence of growth → today's scale → declining feature scarcity → strongest objections → Looski as a measurable experiment.

For each build episode, record the exact workflow replaced, reused foundation, builder/reviewer hours, AI spend, external-service costs, installation effort, operational issues, backup/restore result, and user acceptance. A sustained replacement is stronger evidence than a lookalike screen. One-day builds and five-minute installs remain targets until demonstrated.

Possible titles: **“Why I'm Betting Against Software Subscriptions”** or **“SaaS Is Going to Zero? I'm Building the Test.”** If using “SaaS Is Dead,” define the targeted pricing model early and state that it is a thesis being tested.
