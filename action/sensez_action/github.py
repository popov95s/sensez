from __future__ import annotations

import json
import urllib.error
import urllib.parse
import urllib.request
from typing import Any
from typing import Optional


class GitHubError(Exception):
    pass


class GitHubClient:
    def __init__(self, api_url: str, repository: str, token: str) -> None:
        self.api_url = api_url.rstrip("/")
        self.repository = repository
        self.token = token

    def get(self, path: str) -> Any:
        return self._request("GET", path)

    def post(self, path: str, data: dict[str, Any]) -> Any:
        return self._request("POST", path, data)

    def paged(self, path: str) -> list[dict[str, Any]]:
        items: list[dict[str, Any]] = []
        next_path = path
        while next_path:
            response, links = self._request_with_links("GET", next_path)
            items.extend(response)
            next_path = links.get("next")
        return items

    def _request(
        self, method: str, path: str, data: Optional[dict[str, Any]] = None
    ) -> Any:
        response, _ = self._request_with_links(method, path, data)
        return response

    def _request_with_links(
        self, method: str, path: str, data: Optional[dict[str, Any]] = None
    ) -> tuple[Any, dict[str, str]]:
        url = self._url(path)
        body = json.dumps(data).encode("utf-8") if data is not None else None
        request = urllib.request.Request(url, data=body, method=method)
        request.add_header("Accept", "application/vnd.github+json")
        request.add_header("X-GitHub-Api-Version", "2022-11-28")
        if self.token:
            request.add_header("Authorization", f"Bearer {self.token}")
        if body is not None:
            request.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                payload = response.read().decode("utf-8")
                links = _parse_link_header(response.headers.get("Link", ""))
                return json.loads(payload) if payload else None, links
        except urllib.error.HTTPError as error:
            detail = error.read().decode("utf-8", "replace")
            raise GitHubError(f"GitHub API {method} {url} failed: {error.code} {detail}") from error
        except urllib.error.URLError as error:
            raise GitHubError(f"GitHub API {method} {url} failed: {error}") from error

    def _url(self, path: str) -> str:
        if path.startswith("http://") or path.startswith("https://"):
            return path
        return f"{self.api_url}/repos/{self.repository}/{path.lstrip('/')}"


def _parse_link_header(value: str) -> dict[str, str]:
    links: dict[str, str] = {}
    for part in value.split(","):
        section = part.strip()
        if not section:
            continue
        url_part, _, rel_part = section.partition(";")
        rel = rel_part.strip().removeprefix('rel="').removesuffix('"')
        url = url_part.strip().removeprefix("<").removesuffix(">")
        if rel and url:
            links[rel] = url
    return links
