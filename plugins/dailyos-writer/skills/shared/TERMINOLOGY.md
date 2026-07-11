# Terminology

Universal terminology discipline that applies to all content types. This file is intentionally product-agnostic. Add your own project's product names, brand spellings, and internal vocabulary where noted.

## Principles

- **Consistency beats correctness.** Pick one spelling/casing for a term and use it everywhere in a piece.
- **Full name on first reference, short form after.** "Technical Account Manager (TAM)" first, "TAM" after. Same for any acronym or product.
- **Follow official styling** for product, company, and brand names (capitalization, spacing, punctuation). When unsure, check the entity's own site.
- **Neutral competitive language.** Describe alternatives factually ("Adobe Experience Manager", not "AEM's bloated suite"). Avoid loaded adjectives.

## Project-specific terms (fill in)

List the product names, brands, and proper nouns your project cares about, with correct and incorrect forms. Example shape:

```
**ProductName** (not Product Name, productname)
**BrandName** (official styling)
```

The mechanical pass can enforce these once you add them to `scripts/lint_typography.py` → `check_terminology`.

## Roles & engagement types (fill in)

Define the role names, meeting types, and internal vocabulary your team uses, with first-reference and short forms. Keep these out of customer-facing content unless the customer uses them too.

## Geographic & cultural

### Regional spelling
Pick one and hold it per piece:
- **American English**: "organization", "optimize"
- **British/Canadian English**: "organisation"/"organization", "colour", "favour"

### Date formats
- **ISO 8601 for filenames**: YYYY-MM-DD (2026-06-07)
- **Prose**: "June 7, 2026" or "Jun 7"

### Currency
- **State the currency**: $100M (USD), EUR 50M, GBP 40M. Don't leave a bare symbol ambiguous in international contexts.

## Technical terminology (common cases)

- **REST API** (not Rest API, rest api)
- **GraphQL** (not Graph QL, graphQL)
- **webhook** (not WebHook, web hook)
- **CDN** (Content Delivery Network — spell out on first reference)
- **multisite** (not multi-site)
- **subdomain** (not sub-domain)

Add your own technical terms as needed.
