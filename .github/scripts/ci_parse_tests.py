import json
import sys


def _parse_tests(raw: str) -> list[str]:
    raw = raw.strip()
    if not raw:
        return []

    if raw.startswith("["):
        try:
            data = json.loads(raw)
        except json.JSONDecodeError as exc:  # keep message for CI logs
            raise SystemExit(f"Invalid JSON tests input: {exc}")
        if not isinstance(data, list):
            raise SystemExit("Tests JSON must be an array")
        items = [str(item).strip() for item in data if str(item).strip()]
    else:
        items = [part.strip() for part in raw.split(",") if part.strip()]

    return items


def main() -> None:
    raw = sys.argv[1] if len(sys.argv) > 1 else ""
    items = _parse_tests(raw)
    if not items:
        raise SystemExit("No tests provided")

    for item in items:
        print(item)


if __name__ == "__main__":
    main()
