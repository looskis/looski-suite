# SaaS origins and growth: source notes for the Looski video series

Research checked 2026-10-12. Scope: beginnings, definition, adoption and growth; this is not a market valuation or an AI-displacement forecast. Primary documents take precedence over retrospective marketing claims. Dollar figures are nominal US dollars.

## Defensible historical framing

Use **“the modern SaaS wave emerged in the late 1990s”**, then make Salesforce the main character. Do not claim a provable first inventor. Hosted applications and selling access to computing preceded the web; the consequential change was the combination of internet delivery, provider operation and a scalable recurring commercial model.

For this series, use “pure SaaS” as defined in [intent.md](../../intent.md): products whose primary deliverable is application software, including CRM, forms, CMS and project management. The project excludes named infrastructure, payment/carrier, proprietary-data and other categories. This is a project-specific scope, not a formal NIST or analyst category. Separately, a vendor can have started with online delivery or transitioned from installed software; that historical distinction does not define Looski's target market.

| Claim / date | Primary evidence and qualification |
|---|---|
| SaaS is provider-operated application access; a browser is not mandatory. | [NIST SP 800-145, September 2011, p. 2](https://nvlpubs.nist.gov/nistpubs/legacy/sp/nistspecialpublication800-145.pdf) defines access through thin clients **or a program interface**, with underlying infrastructure managed by the provider. Subscription pricing alone is insufficient to identify SaaS. |
| The business-model ancestry predates the web. | [Computer History Museum, Service Bureau Pioneer History Meeting, 2010, p. 37](https://archive.computerhistory.org/resources/access/text/2016/10/102743008-05-01-acc.pdf): participants describe earlier service bureaus and time-sharing/remote-processing firms running applications for customers, and connect that model to SaaS/cloud. This is participant oral history, not proof that early time-sharing satisfies today's cloud definition. |
| Concur was founded in 1993, but did not begin with online application delivery. | [Concur FY1999 Form 10-K](https://www.sec.gov/Archives/edgar/data/1066026/000103221099001811/0001032210-99-001811-d1.html): operations began FY1994; initial QuickXpense was shrink-wrapped software, first shipped FY1995. Client-server Concur Expense followed in July 1996 and an intranet version in March 1998. By this filing, Concur offered licensed, ASP and internet-outsourced editions; the latter two used subscription pricing. eWorkplace.com was introduced October 1999. Do not equate the 1993 company founding with a SaaS launch. |
| NetSuite dates to September 1998; NetLedger's first release was in 1999. | [NetSuite FY2007 Form 10-K, Business/Overview](https://www.sec.gov/Archives/edgar/data/1117106/000119312508065855/d10k.htm). Prefer this contemporaneous disclosure over [the current NetSuite marketing timeline](https://www.netsuite.com/portal/aboutus.shtml), which labels 1998 as NetLedger's launch. |
| Salesforce was founded in 1999 and offered its service from February 2000. | [Salesforce April 20, 2004 S-1/A, Summary and Risk Factors](https://www.sec.gov/Archives/edgar/data/1108524/000119312504065496/ds1a.htm). The filing describes browser-delivered CRM sold by subscription, with a multi-tenant architecture. Its reported founding/incorporation month is February; [the retrospective company history](https://www.salesforce.com/news/stories/the-history-of-salesforce/) gives March 8 for incorporation. Say **1999** rather than resolving this discrepancy by guesswork. |

## A chart that demonstrates the growth

Use Salesforce **total annual revenue**, consistently labeled. These are not market capitalization, ARR or total SaaS industry revenues. FY labels below mean the year ended January 31 of the displayed year.

| Fiscal year / period end | Total revenue | Source |
|---|---:|---|
| FY2004 / 2004-01-31 | $96.023 million | [2004 S-1/A, financial statements](https://www.sec.gov/Archives/edgar/data/1108524/000119312504065496/ds1a.htm) |
| FY2009 / 2009-01-31 | $1.076769 billion | [FY2009 10-K, consolidated statements](https://www.sec.gov/Archives/edgar/data/1108524/000119312509048665/d10k.htm) |
| FY2014 / 2014-01-31 | $4.071003 billion | [FY2016 10-K, comparative consolidated statements](https://www.sec.gov/Archives/edgar/data/1108524/000110852416000093/R4.htm) |
| FY2019 / 2019-01-31 | $13.282 billion | [FY2020 10-K, comparative quarterly financial information](https://www.sec.gov/Archives/edgar/data/1108524/000110852420000014/R41.htm) |
| FY2026 / 2026-01-31 | $41.525 billion | [February 25, 2026 earnings release, consolidated statements](https://investor.salesforce.com/news/news-details/2026/Salesforce-Delivers-Record-Fourth-Quarter-Fiscal-2026-Results/default.aspx) |

Computed from the first two rows: **11.21× revenue in five years**, or **62.16% compound annual growth**, using `(1,076.769 / 96.023)^(1/5) - 1`. This is a company growth example, not an industry CAGR. The long series reflects a changing product portfolio and acquisitions, not only organic growth in the original CRM product. FY2026 included $399 million of Informatica revenue, per the earnings release; its full-year results were up approximately 10% year over year.

For a second early business: [NetSuite's FY2008 10-K](https://www.sec.gov/Archives/edgar/data/1117106/000119312509053860/d10k.htm) reports total revenue rising from **$17.7 million in calendar 2004 to $152.5 million in calendar 2008**. Computed from these rounded reported figures: **8.62× in four years**, approximately **71.3% CAGR**. Do not align these December year-ends with Salesforce's January year-ends without labeling them.

## Why adoption could accelerate

Contemporaneous evidence supports several mechanisms, rather than a claim that subscriptions alone caused growth:

- Salesforce's [2004 prospectus, Summary and Industry Background](https://www.sec.gov/Archives/edgar/data/1108524/000119312504065496/ds1a.htm) cites wider internet access and cheaper bandwidth/computing. Customers could avoid installing infrastructure; the company argued this reduced upfront cost and deployment effort. It distinguished its shared application architecture from first-generation ASPs hosting separate copies of third-party applications. Attribute those comparative advantages to Salesforce's explanation, not an independent causal experiment.
- Salesforce's [FY2009 10-K, revenue discussion](https://www.sec.gov/Archives/edgar/data/1108524/000119312509048665/d10k.htm) attributes subscription growth to new customers, extra subscriptions within existing customers, and upgrades. It reports **55,400 customers at January 31, 2009**, compared with **41,000** a year before. Subscription/support revenue was **$984.574 million**, or approximately **91%** of total revenue in FY2009. Its AppExchange and developer platform also enabled third-party applications.
- Broader adoption continued beyond the early winners: [Eurostat, published February 3, 2026](https://ec.europa.eu/eurostat/web/products-eurostat-news/w/ddn-20260203-1), reports paid cloud-service use by EU enterprises increasing from **17.8% in 2014 to 52.7% in 2025**. Explicitly label this **cloud adoption, not SaaS-only adoption**; it includes software, storage and computing power. The [underlying dataset](https://ec.europa.eu/eurostat/databrowser/view/ISOC_CICCE_USE/default/table?lang=en) covers enterprises with at least 10 people employed in its specified economic sectors, not every business or household. The increase is 34.9 percentage points, not 34.9%.

## Suggested narrative and guardrails

Narrative synthesis: Businesses already paid outsiders to run computing. The web made online business applications more accessible, and late-1990s companies turned that opportunity into subscription businesses. One of the defining examples grew more than elevenfold in five years. The resulting category became large enough that changes to its pricing and distribution now matter enormously.

Keep these distinctions visible in the script:

- “Salesforce helped popularize SaaS” is defensible; “Salesforce invented SaaS” overstates the evidence.
- A company founding date, product release, internet/intranet version and subscription transition are different events.
- A subscription to installed software is not automatically SaaS; an API-delivered service is not automatically outside SaaS.
- Company revenue growth proves commercial traction, not a universal winner or investment return. Growth and profitability are separate metrics.
- Nothing in this historical record establishes that SaaS will go to zero. That is a separate forward-looking thesis requiring explicit mechanisms, counterevidence and a definition of what is allegedly disappearing.
