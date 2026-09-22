# CRM denomination catalog

CRM estimates use decimal strings backed by PostgreSQL `numeric(20,6)`: up to fourteen integer and six fractional digits, nonnegative, with a paired denomination. No binary floating point, silent rounding, FX conversion or ledger semantics are provided.

`crates/looski/src/crm/currencies.txt` pins 305 distinct current or historical ISO 4217 currency/fund codes, excluding the testing code XTS and the no-currency code XXX. Historical codes are accepted because deals can represent past transactions. This is a versioned supported catalog, not a runtime network dependency or a promise that every listed currency is currently in circulation.

Retrieved October 12, 2026 from the [SIX ISO 4217 maintenance agency](https://www.six-group.com/en/products-services/financial-information/market-reference-data/data-standards.html): [List One XML](https://www.six-group.com/dam/download/financial-information/data-center/iso-currrency/lists/list-one.xml), published 2026-09-17, and [List Three XML](https://www.six-group.com/dam/download/financial-information/data-center/iso-currrency/lists/list-three.xml), published 2026-01-01. Codes are the sorted union of nonempty three-character `Ccy` values minus XTS/XXX. Future catalog updates require an explicit reviewed source refresh and validation; removing a denomination must not strand existing historical records.
