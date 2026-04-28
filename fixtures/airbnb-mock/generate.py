#!/usr/bin/env python3
"""Mock Airbnb-like listings using airbert-vln/bnb-dataset images + synthesized structured fields.

Image source: https://github.com/airbert-vln/bnb-dataset (CC images Airbnb collected ~Christmas 2019)
Image URLs are direct `a0.muscache.com` CDN links.

Output:
    fixtures/airbnb-mock/listings.json — TARGET_COUNT mock listings, deterministic via SEED.

Usage:
    cd <repo_root>
    python3 fixtures/airbnb-mock/generate.py [--count N] [--cache <tsv_path>]

Reproducible — same SEED always produces the same listings.json.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import random
import sys
import urllib.request
from collections import defaultdict
from pathlib import Path

SEED = 20260428
DEFAULT_COUNT = 500
MIN_PHOTOS = 6
MAX_PHOTOS = 10  # truncate gallery for demo

DEFAULT_TSV_URL = (
    "https://media.githubusercontent.com/media/airbert-vln/bnb-dataset/"
    "refs/heads/main/data/airbnb-test-indoor-filtered.tsv"
)
# 5 MB byte range covers > 5000 listings — far more than we need.
TSV_BYTE_RANGE = 5_000_000

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE_DIR = Path(__file__).resolve().parent
OUT_PATH = FIXTURE_DIR / "listings.json"
DEFAULT_CACHE = FIXTURE_DIR / ".tsv-cache.tsv"

CITIES = [
    {
        "city": "Beijing", "country": "China",
        "currency": "CNY", "fx_to_usd": 0.14,
        "lat": 39.9042, "lon": 116.4074, "spread": 0.10,
        "neighborhoods": ["Sanlitun", "Gulou", "Wangfujing", "Wudaokou", "CBD", "Houhai"],
        "weight": 0.18, "price_lo": 280, "price_hi": 1200,
    },
    {
        "city": "Tokyo", "country": "Japan",
        "currency": "JPY", "fx_to_usd": 0.0067,
        "lat": 35.6762, "lon": 139.6503, "spread": 0.08,
        "neighborhoods": ["Shinjuku", "Shibuya", "Asakusa", "Roppongi", "Ueno", "Akihabara"],
        "weight": 0.22, "price_lo": 8000, "price_hi": 35000,
    },
    {
        "city": "New York", "country": "USA",
        "currency": "USD", "fx_to_usd": 1.0,
        "lat": 40.7128, "lon": -74.0060, "spread": 0.06,
        "neighborhoods": ["SoHo", "Williamsburg", "Upper West Side", "Harlem", "East Village", "Astoria"],
        "weight": 0.20, "price_lo": 90, "price_hi": 380,
    },
    {
        "city": "Paris", "country": "France",
        "currency": "EUR", "fx_to_usd": 1.07,
        "lat": 48.8566, "lon": 2.3522, "spread": 0.05,
        "neighborhoods": ["Le Marais", "Montmartre", "Saint-Germain", "Latin Quarter", "Bastille", "Belleville"],
        "weight": 0.15, "price_lo": 70, "price_hi": 320,
    },
    {
        "city": "Bangkok", "country": "Thailand",
        "currency": "THB", "fx_to_usd": 0.029,
        "lat": 13.7563, "lon": 100.5018, "spread": 0.10,
        "neighborhoods": ["Sukhumvit", "Silom", "Khao San", "Riverside", "Chinatown", "Ari"],
        "weight": 0.12, "price_lo": 800, "price_hi": 4500,
    },
    {
        "city": "Lisbon", "country": "Portugal",
        "currency": "EUR", "fx_to_usd": 1.07,
        "lat": 38.7223, "lon": -9.1393, "spread": 0.06,
        "neighborhoods": ["Alfama", "Bairro Alto", "Chiado", "Belém", "Príncipe Real", "Graça"],
        "weight": 0.13, "price_lo": 55, "price_hi": 240,
    },
]

ROOM_TYPES = [
    ("Entire place", 0.60),
    ("Private room", 0.33),
    ("Shared room", 0.07),
]

AMENITY_POOL = [
    "Wi-Fi", "Kitchen", "Heating", "Air conditioning", "Washer", "Dryer",
    "TV", "Free parking", "Workspace", "Pool", "Gym", "Hot tub",
    "Pet-friendly", "Self check-in", "Elevator", "Iron", "Hair dryer",
    "Smoke alarm", "Carbon monoxide alarm", "First aid kit",
    "Coffee maker", "Dishwasher", "Refrigerator", "Microwave",
    "Balcony", "Garden", "EV charger", "Crib",
]
ESSENTIAL_AMENITIES = ["Wi-Fi", "Heating"]

HOST_FIRST_NAMES = [
    "Akira", "Yuki", "Mei", "Wei", "Liang", "Hiroshi", "Sara", "Maya", "Sofia",
    "Marco", "Elena", "Diego", "Liam", "Olivia", "Noah", "Emma", "Lucas",
    "Pierre", "Camille", "Henrique", "Beatriz", "Niran", "Suthida",
    "Alex", "Jordan", "Sam", "Charlie", "Ana", "Mateus",
]

TITLE_TEMPLATES = {
    "Entire place": [
        "Sunny {n} apartment in {nbhd}",
        "Cozy {n} loft near {nbhd}",
        "Modern {n} studio steps from {nbhd}",
        "Quiet {n} flat in the heart of {nbhd}",
        "Charming {n} home in historic {nbhd}",
        "Stylish {n} suite overlooking {nbhd}",
    ],
    "Private room": [
        "Private bedroom in {nbhd} share",
        "Comfy room in {nbhd} family home",
        "Quiet private room near {nbhd} station",
        "Bright private room in {nbhd}",
    ],
    "Shared room": [
        "Friendly shared room in {nbhd}",
        "Backpacker dorm bed in {nbhd}",
    ],
}

DESCRIPTION_SNIPPETS = [
    "Walk to cafes, restaurants and the subway in minutes.",
    "Quiet street, great for remote work.",
    "Newly renovated with contemporary furnishings.",
    "Local market and bakery just around the corner.",
    "Plenty of natural light throughout the day.",
    "Comfortable beds and high-quality linens.",
    "Fast Wi-Fi rated for video calls.",
    "Self check-in via smart lock — arrive any time.",
    "Family-friendly, with crib and high chair available on request.",
    "Easy access to airport via direct train line.",
]


def weighted_choice(rng_, choices):
    total = sum(w for _, w in choices)
    r = rng_.random() * total
    upto = 0
    for c, w in choices:
        upto += w
        if upto >= r:
            return c
    return choices[-1][0]


def stable_int(s, mod=1_000_000):
    return int(hashlib.md5(s.encode()).hexdigest(), 16) % mod


def fetch_tsv(cache_path: Path) -> Path:
    if cache_path.exists() and cache_path.stat().st_size > 0:
        return cache_path
    print(f"[fetch] downloading {TSV_BYTE_RANGE // 1_000_000} MB from {DEFAULT_TSV_URL}", file=sys.stderr)
    req = urllib.request.Request(
        DEFAULT_TSV_URL,
        headers={"Range": f"bytes=0-{TSV_BYTE_RANGE - 1}"},
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        data = resp.read()
    cache_path.write_bytes(data)
    return cache_path


def load_listings(tsv_path: Path):
    by_id = defaultdict(list)
    with open(tsv_path) as f:
        for r in csv.reader(f, delimiter="\t"):
            if len(r) >= 4 and r[0].isdigit():
                listing_id, photo_id, url, caption = r[0], r[1], r[2], r[3]
                by_id[listing_id].append({
                    "photo_id": photo_id,
                    "url": url,
                    "caption": caption.strip() or None,
                })
    return by_id


def build_listing(listing_id: str, photos: list, rng: random.Random) -> dict:
    photos = photos[:MAX_PHOTOS]

    s = stable_int(listing_id + ":city") / 1_000_000
    upto, city = 0.0, None
    for c in CITIES:
        upto += c["weight"]
        if s <= upto:
            city = c
            break
    if city is None:
        city = CITIES[-1]

    nbhd = rng.choice(city["neighborhoods"])
    room_type = weighted_choice(rng, ROOM_TYPES)
    bedrooms = rng.choice([0, 1, 1, 1, 2, 2, 3]) if room_type == "Entire place" else rng.choice([1, 1, 1])
    beds = max(1, bedrooms + rng.choice([-1, 0, 0, 1]))
    bathrooms = round(rng.choice([1, 1, 1, 1.5, 2, 2.5, 3]), 1)
    accommodates = max(1, min(8, bedrooms * 2 + rng.choice([0, 0, 1, 1, 2])))
    if room_type == "Private room":
        accommodates = min(accommodates, 2)
        bedrooms = 1
    if room_type == "Shared room":
        accommodates = 1
        bedrooms = 0
        beds = 1

    price_local = rng.randint(city["price_lo"], city["price_hi"])
    price_usd = round(price_local * city["fx_to_usd"], 2)

    rating = round(rng.gauss(4.78, 0.18), 2)
    rating = max(3.8, min(5.0, rating))
    review_count = max(1, int(rng.lognormvariate(4.0, 0.9)))

    n_amen = rng.randint(5, 12)
    amen_pool = [a for a in AMENITY_POOL if a not in ESSENTIAL_AMENITIES]
    rng.shuffle(amen_pool)
    amenities = ESSENTIAL_AMENITIES + amen_pool[: n_amen - len(ESSENTIAL_AMENITIES)]

    title_template = rng.choice(TITLE_TEMPLATES[room_type])
    nbed_str = f"{bedrooms}-bedroom" if bedrooms >= 1 else "studio"
    title = title_template.format(n=nbed_str, nbhd=nbhd)
    descr = " ".join(rng.sample(DESCRIPTION_SNIPPETS, k=rng.randint(2, 4)))

    lat = round(city["lat"] + rng.uniform(-city["spread"], city["spread"]), 6)
    lon = round(city["lon"] + rng.uniform(-city["spread"], city["spread"]), 6)
    minimum_nights = rng.choice([1, 1, 1, 2, 2, 3, 7])

    return {
        "listing_id": listing_id,
        "title": title,
        "description": descr,
        "room_type": room_type,
        "city": city["city"],
        "country": city["country"],
        "neighborhood": nbhd,
        "lat": lat, "lon": lon,
        "accommodates": accommodates,
        "bedrooms": bedrooms,
        "beds": beds,
        "bathrooms": bathrooms,
        "amenities": amenities,
        "minimum_nights": minimum_nights,
        "price": {
            "amount": price_local,
            "currency": city["currency"],
            "amount_usd": price_usd,
            "per": "night",
        },
        "rating": rating,
        "review_count": review_count,
        "host": {
            "name": rng.choice(HOST_FIRST_NAMES),
            "is_superhost": rng.random() < 0.30,
        },
        "photos": photos,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--count", type=int, default=DEFAULT_COUNT)
    parser.add_argument("--cache", type=Path, default=DEFAULT_CACHE)
    parser.add_argument("--out", type=Path, default=OUT_PATH)
    args = parser.parse_args()

    tsv_path = fetch_tsv(args.cache)
    by_id = load_listings(tsv_path)
    candidates = sorted(
        [(lid, ph) for lid, ph in by_id.items() if len(ph) >= MIN_PHOTOS],
        key=lambda kv: kv[0],
    )
    rng = random.Random(SEED)
    rng.shuffle(candidates)
    if len(candidates) < args.count:
        sys.exit(f"only {len(candidates)} listings have >= {MIN_PHOTOS} photos in cached TSV; "
                 f"increase TSV_BYTE_RANGE or lower --count")
    selected = candidates[: args.count]

    out = [build_listing(lid, photos, rng) for lid, photos in selected]
    args.out.write_text(json.dumps(out, indent=2, ensure_ascii=False))

    cities_dist = defaultdict(int)
    rt_dist = defaultdict(int)
    for l in out:
        cities_dist[l["city"]] += 1
        rt_dist[l["room_type"]] += 1
    print(f"wrote {len(out)} listings → {args.out.relative_to(REPO_ROOT)}")
    print(f"file size: {args.out.stat().st_size:,} bytes")
    print(f"city distribution:      {dict(cities_dist)}")
    print(f"room_type distribution: {dict(rt_dist)}")


if __name__ == "__main__":
    main()
