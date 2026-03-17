# GitHub API Quick Reference

## Base URL
```
https://api.github.com
```

## Authentication (Optional)

```bash
# Without token (60 requests/hour)
curl https://api.github.com/users/username

# With token (5000 requests/hour)
curl -H "Authorization: Bearer YOUR_TOKEN" https://api.github.com/users/username
```

## Key Endpoints

### Repository Information

**Get Repository Details**
```bash
GET /repos/{owner}/{repo}
curl https://api.github.com/repos/vercel/next.js
```

**Response Fields:**
- `name`: Repository name
- `full_name`: owner/repo
- `description`: Project description
- `language`: Primary language
- `stargazers_count`: Stars
- `forks_count`: Forks
- `open_issues_count`: Open issues
- `topics`: Repository topics

### README

**Get README**
```bash
GET /repos/{owner}/{repo}/readme
curl https://api.github.com/repos/vercel/next.js/readme
```

**Parameters:**
- `ref`: Branch name (default: main)

**Response:**
- Base64 encoded content in `content` field

### Repository Contents

**Get Directory/File**
```bash
GET /repos/{owner}/{repo}/contents/{path}?ref={branch}
curl https://api.github.com/repos/vercel/next.js/contents/
```

**Get File Tree (Recursive)**
```bash
GET /repos/{owner}/{repo}/git/trees/{branch}?recursive=1
curl https://api.github.com/repos/vercel/next.js/git/trees/main?recursive=1
```

### Rate Limits

**Headers:**
```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 57
X-RateLimit-Reset: 1372700873
```

**Limits:**
- Unauthenticated: 60/hour
- Authenticated: 5000/hour
- Search: 30/minute

## Common Mirror Endpoints

| Mirror | Base URL | Notes |
|--------|----------|-------|
| Official | api.github.com | Best reliability |
| gh-proxy | gh-proxy.com/api/github | China-optimized |
| FastGit | api.fastgit.org | Good fallback |
| kgithub | api.kgithub.com | Another option |

## curl Examples

```bash
# Get repo info
curl -s https://api.github.com/repos/vercel/next.js | jq '.'

# Get README (decode base64)
curl -s https://api.github.com/repos/vercel/next.js/readme | \
  jq -r '.content' | base64 -d

# Get file tree
curl -s "https://api.github.com/repos/vercel/next.js/git/trees/main?recursive=1" | \
  jq '.tree[] | .path, .type'

# Get specific file
curl -s https://api.github.com/repos/vercel/next.js/contents/package.json | \
  jq -r '.content' | base64 -d
```

## Error Codes

| Code | Meaning |
|------|---------|
| 200 | Success |
| 404 | Not Found |
| 403 | Forbidden (rate limit) |
| 422 | Validation Error |

## Branch Detection

Common branch names to try:
- `main` (default)
- `master`
- `develop`
- `dev`
