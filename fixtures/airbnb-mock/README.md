# Airbnb-mock fixture

Mock listing data for the in-development `booking` agent2app capability
(see `specs/task-agent-to-app-l2a-booking-capability.spec.md`).

## What's here

- `listings.json` — 500 deterministic mock listings. Real images from the
  airbert-vln dataset; structured fields (price, city, capacity, rating,
  amenities, host, etc.) synthesized via fixed RNG seed.
- `generate.py` — regenerates `listings.json`. Idempotent: same seed →
  identical output.
- `.tsv-cache.tsv` — local cache of the 5 MB byte range pulled from the
  upstream TSV. Recreated automatically; gitignored.

## Provenance

- **Images**: airbert-vln/bnb-dataset
  (https://github.com/airbert-vln/bnb-dataset, captured ~Christmas 2019).
  Image URLs point directly to Airbnb's CDN (`a0.muscache.com`).
  Subset: `airbnb-test-indoor-filtered.tsv`. Indoor-filtered, captioned.
- **Structured fields**: synthesized in this script. **Not real Airbnb data**.
  Treat as fixture/demo only.

## Schema (per listing)

```json
{
  "listing_id": "10015654",
  "title": "Cozy 2-bedroom loft near SoHo",
  "description": "Walk to cafes, restaurants and the subway in minutes. ...",
  "room_type": "Entire place" | "Private room" | "Shared room",
  "city": "New York", "country": "USA",
  "neighborhood": "SoHo",
  "lat": 40.71, "lon": -74.00,
  "accommodates": 4, "bedrooms": 2, "beds": 2, "bathrooms": 1.5,
  "amenities": ["Wi-Fi", "Heating", ...],
  "minimum_nights": 2,
  "price": {
    "amount": 220, "currency": "USD",
    "amount_usd": 220.0, "per": "night"
  },
  "rating": 4.86, "review_count": 142,
  "host": { "name": "Alex", "is_superhost": true },
  "photos": [
    { "photo_id": "104324945",
      "url": "https://a0.muscache.com/pictures/<uuid>.jpg",
      "caption": "Kitchen - Full Service" }
  ]
}
```

## Distribution (default 500-listing build)

- **Cities**: Beijing 85, Bangkok 63, Tokyo 95, Lisbon 68, Paris 93,
  New York 96
- **Room types**: Entire place 294 (59%), Private room 167 (33%),
  Shared room 39 (8%)
- **Photos/listing**: 6–10 (truncated for demo)
- **Ratings**: 3.8–5.0, normal-distributed around 4.78
- **Currencies**: per-city local (CNY, JPY, USD, EUR, THB, EUR)
  with `amount_usd` provided for cross-city sorting

## Regenerate

```bash
# default 500 listings
python3 fixtures/airbnb-mock/generate.py

# arbitrary size (the upstream TSV byte-range covers thousands of listings)
python3 fixtures/airbnb-mock/generate.py --count 1000
```

The script downloads ~5 MB from GitHub LFS on first run, then caches it
under `.tsv-cache.tsv`. Subsequent regenerations are offline.

## Caveats

- Image URLs are from 2019; some may have been deleted by Airbnb. Production
  use should mirror needed images to a self-hosted CDN.
- All non-image fields are synthetic. Do not use for any real-money flow.
- The `listing_id` values are real Airbnb IDs from the 2019 snapshot — they
  identify the image bundle, not a current real listing.
