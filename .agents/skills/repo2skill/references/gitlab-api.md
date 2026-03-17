# GitLab API Quick Reference

## Base URL
```
https://gitlab.com/api/v4
```

## Authentication

```bash
# Without token (60 requests/minute)
curl https://gitlab.com/api/v4/projects

# With private token (unlimited)
curl -H "PRIVATE-TOKEN: YOUR_TOKEN" https://gitlab.com/api/v4/projects
```

## Key Endpoints

### Get Project by Path

```bash
GET /projects/:id
# ID can be encoded path or numeric ID

# Example with path (URL encoded)
curl "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab"

# Alternative with raw path
curl "https://gitlab.com/api/v4/projects?search=gitlab" | jq '.[] | select(.path == "gitlab")'
```

### Get Repository Files

**Get File Tree**
```bash
GET /projects/:id/repository/tree
curl "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/repository/tree?recursive=1"
```

**Get File Content**
```bash
GET /projects/:id/repository/files/:file_path/raw?ref=branch

curl "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/repository/files/README.md/raw?ref=main"
```

**Get README**
```bash
# Similar to file content
curl "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/repository/files/README.md/raw?ref=main"
```

## Rate Limits

**Limits:**
- Unauthenticated API: ~60 requests/minute
- Authenticated: ~600 requests/minute (varies by tier)

**Headers:**
```
RateLimit-Limit: 600
RateLimit-Observed: 12
RateLimit-Remaining: 588
RateLimit-Reset: 1234567890
```

## Project Encoding

GitLab uses URL-encoded paths for project IDs:

| Actual Path | Encoded Path |
|-------------|--------------|
| gitlab-org/gitlab | gitlab-org%2Fgitlab |
| namespace/project | namespace%2Fproject |

## curl Examples

```bash
# Get project info (search method recommended)
curl -s "https://gitlab.com/api/v4/projects?search=gitlab" | \
  jq '.[] | select(.path == "gitlab")'

# Get README
curl -s "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/repository/files/README.md/raw?ref=main"

# Get file tree
curl -s "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/repository/tree?recursive=1" | \
  jq '.[] | .path, .type'

# Get specific file
curl -s "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab/repository/files/package.json/raw?ref=master"

# With authentication
curl -H "PRIVATE-TOKEN: YOUR_TOKEN" \
  "https://gitlab.com/api/v4/projects/gitlab-org%2Fgitlab"
```

## Branch Detection

Common branch names:
- `main` (default) 
- `master`
- `develop`
- `stable`

## Differences from GitHub

| Feature | GitHub | GitLab |
|---------|--------|--------|
| Project ID | owner/repo | URL encoded ID or numeric |
| Files Endpoint | /contents/ | /repository/files/:path/raw |
| README | /readme | /repository/files/README.md/raw |
| Tree | /git/trees | /repository/tree |
