# Licensed entry guide

> Updated: 2026-10-19
> Scope: a guide for players in mainland China who want to buy licensed Avatars and materials
> Status: Draft

## Purpose

Many players in mainland China want to use licensed Avatars and materials and have had no
practical way in: a pixiv account, BOOTH's prices, a card that pays abroad and the purchase itself
are each a hurdle. VUA offers them a path, following the author's ruling of 2026-10-19 in the
[product boundary](../product-boundary.md#production-scope-and-product-rulings-user-ruling-2026-09-22).
It serves the same Japanese creators whose community VUA is built for: every completed purchase
supports them.

The guide follows two rules from the product boundary:

- **Restraint.** VUA explains, links to the official pages and checks what it can observe. Sign-in,
  card applications and payment stay with the player on the official sites.
- **Tolerance.** The guide is reached by the player's choice, from Help and the library. What a
  player's library contains never opens it, and its text celebrates creators and the benefits of
  buying; it makes no comparison with other sources.

## Contents

Each part is a [versioned SOP](../north-star.md#sops) in the four UI languages, with the date,
network and software versions of its last verification. Facts that change often (fees, accepted
cards, sign-up steps) are written only after verification and show their verification date.

| Part | Content |
| --- | --- |
| 1. pixiv account | What pixiv and BOOTH are, why BOOTH needs a pixiv account, the sign-up steps on the official page, two-step verification |
| 2. BOOTH prices | Prices in Japanese yen including consumption tax; digital delivery and the download page; one product with several variants and its supported Avatars (対応アバター); free items; the optional boost; the purchase history VUA's library reads |
| 3. Paying abroad | What a card needs to pay a Japanese shop: an international network such as Visa or Mastercard, foreign-currency settlement and its fees, the yen exchange rate; the questions to ask a bank; no bank or product is recommended |
| 4. Buying | BOOTH's checkout with the card or PayPal as BOOTH currently accepts them, the confirmation mail, then VUA's library downloading the purchase |
| 5. Terms of use | Reading a product's terms, such as the VN3 license and a creator's own 利用規約: what may be changed, uploaded and shared, and what stays forbidden; the terms page linked from each library entry |
| 6. Why licensed | Text, images and short animations about the creators behind each Avatar and outfit, updates and support from the creator, and the community around a base Avatar |

## Network reachability

Some steps need sites that may not respond from mainland China, pixiv among them. VUA keeps its
existing rule for these cases (the "Network and activation" row of
[broader N1 acceptance](../development-outline.md#broader-n1-acceptance-after-the-first-play-release)): it checks the sites a step needs and reports what it observed, and it names no
accelerator, proxy or other network product.

- The guide adds a check for the sites its steps use (pixiv, BOOTH, the card's payment page,
  PayPal), built on the [website tests](../protocols/website-tests-v0.1.md) of the Play network
  tile. A result is an observation for the player, not a verdict on the guide.
- Each step names the sites it needs, so a player whose check fails knows which step it affects.
- Meta Quest's first activation follows the same rule in N1's device preparation.

Verification runs on the author's remote host on a mainland China network, as for
[C-01](../eval/cases.md).

## Creator credits

The [release room](current-plan.md#37--creator-credits) lists, for each finished Avatar, the
materials it uses with their creators and BOOTH pages, from the Build Record's provenance. This
gives creators visible credit wherever a finished Avatar appears in VUA.

## Open questions for the author

- Which sites a mainland check reports, after the author's verification of pixiv, BOOTH and
  PayPal from the remote host.
- How a step reads when its site does not respond from the player's network.
- Whether the guide has its own entry on the base map or lives in Help and the library only.
