
- [P2] Invoke a registered receipt path for claim rows — /Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/blocks/_shared/envelope/envelope-resolver.php:292-292
  This helper sends claim receipt requests through the surface ability endpoint, but the ability registry has no `claim_receipt` entry; receipt rendering exists as the Tauri command `render_claim_receipt` instead. Any inner block that resolves a `claim_ref` through this path gets an ability-unavailable/error response and renders without the trust/provenance receipt data.
