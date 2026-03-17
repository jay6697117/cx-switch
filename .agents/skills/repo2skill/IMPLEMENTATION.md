# Implementation Guide for repo2skill

This document explains how this skill works internally.

## Architecture

### Skill Structure
```
repo2skill/
├── SKILL.md              # Main skill definition (what you're reading)
├── references/           # API reference documents
│   ├── github-api.md
│   ├── gitlab-api.md
│   └── gitee-api.md
└── scripts/
    ├── utils.sh         # Helper bash functions
    └── .gitkeep
```

### How the Skill Executes

When a user asks to convert a repository or local path:

#### Remote Repository Flow

1. **Input Detection** (Step 0)
   - Distinguishes between remote URL and local path
   - Validates input format

2. **URL Parsing** (Step 1)
   - Detects platform (github/gitlab/gitee)
   - Extracts owner and repo name
   - Validates URL format

3. **Data Collection** (Step 2-3)
   - (Tool: webfetch / bash curl)
   - Fetches repository metadata via API
   - Gets README content
   - Retrieves file tree
   - Extracts key documentation files

   Mirror rotation is built into the skill via retry logic

4. **AI Analysis** (Step 5)
   - (Tool: Configured LLM in OpenCode/Claude Code)
   - Analyzes README and gathered content
   - Understands project structure
   - Identifies key features
   - Generates comprehensive documentation

5. **Skill Generation** (Step 6-8)
   - (Tool: write)
   - Builds YAML frontmatter
   - Creates markdown sections
   - Writes SKILL.md to chosen location

#### Local Repository Flow (New)

1. **Input Detection** (Step 0)
   - Identifies local path input (./, /, ~, or existing directory)
   - Validates path exists

2. **Path Validation** (Step 3B.1)
   - Checks directory exists and is readable
   - Gets absolute path
   - Extracts project name

3. **File Extraction** (Step 3B.2-3B.6)
   - (Tools: read, glob, grep, bash)
   - Finds README files
   - Locates configuration files (package.json, requirements.txt, etc.)
   - Extracts documentation
   - Builds project structure understanding

4. **Metadata Inference** (Step 3B.4)
   - Detects project language from config files
   - Extracts description from README
   - Analyzes dependencies
   - Identifies project type

5. **AI Analysis** (Step 5)
   - (Tool: Configured LLM in OpenCode/Claude Code)
   - Analyzes extracted local files
   - Understands project structure
   - Generates comprehensive documentation

6. **Skill Generation** (Step 6-8)
   - (Tool: write)
   - Builds YAML frontmatter with local metadata
   - Creates markdown sections
   - Writes SKILL.md to chosen location

## Implementation Details

### Mirror Rotation Logic

The skill instructs the AI to:
1. Try primary endpoint (api.github.com)
2. On failure (403/429/timeout), try next mirror
3. Use exponential backoff between retries
4. Retry up to 5 times per endpoint

Mirror priority is defined in references/github-api.md

### Rate Limit Handling

- Headers checked: X-RateLimit-Remaining, X-RateLimit-Reset
- When limit reached: switch mirrors or wait
- Wait time calculated from reset timestamp

### LLM Integration

The skill does NOT hardcode LLM provider:
- Uses whatever LLM is configured in OpenCode/Claude Code
- No API keys needed in the skill
- Works with Claude, GPT, Ollama, local models

### Tool Usage

Only built-in OpenCode/Claude Code tools are used:

| Tool | Purpose | Remote Flow | Local Flow |
|------|---------|-------------|------------|
| `skill` | Load this skill's instructions | ✅ | ✅ |
| `webfetch` | Fetch remote API responses | ✅ | ❌ |
| `bash` | Execute commands (curl, path operations) | ✅ | ✅ |
| `read` | Read files from disk | ✅ | ✅ (local files) |
| `glob` | Find files by pattern | ❌ | ✅ (local discovery) |
| `grep` | Search content in files | ❌ | ✅ (pattern analysis) |
| `write` | Write generated SKILL.md | ✅ | ✅ |
| Configured LLM | Analyze and generate content | ✅ | ✅ |

## For Users Installing This Skill

### What Happens When You Use It

#### Remote Repository Example

```
User: "帮我把这个仓库转成技能：https://github.com/user/repo"
```

**AI Process:**
1. Loads repo2skill skill instructions
2. Detects input type → Remote URL (Step 0)
3. Parses URL → detects GitHub, owner=user, repo=repo (Step 1)
4. Calls webfetch/bash to get repo data from mirrors (Steps 2-3)
5. Retrieves README, metadata, file tree
6. Uses configured LLM (Claude/GPT/etc.) to analyze (Step 5)
7. Generates comprehensive SKILL.md with:
   - YAML frontmatter
   - Overview and features
   - Installation guide
   - Usage examples
   - API reference
   - Troubleshooting
   - Resources
8. Asks user where to save:
   - `./.opencode/skills/repo-name/SKILL.md`
   - `~/.config/opencode/skills/repo-name/SKILL.md`
   - `~/.claude/skills/repo-name/SKILL.md`
9. Writes the file
10. Done!

#### Local Repository Example (New)

```
User: "帮我把当前项目转成技能"
     OR
User: "帮我把这个本地项目转成技能：./my-project"
```

**AI Process:**
1. Loads repo2skill skill instructions
2. Detects input type → Local path (Step 0)
3. Validates path → exists and is readable (Step 3B.1)
4. Uses glob/grep/read to extract local files (Step 3B.3):
   - Finds README files
   - Locates config files (package.json, requirements.txt, etc.)
   - Builds project structure understanding
5. Infers metadata from local files (Step 3B.4):
   - Detects language from config
   - Extracts description from README
   - Analyzes dependencies
6. Uses configured LLM (Claude/GPT/etc.) to analyze (Step 5)
7. Generates comprehensive SKILL.md with:
   - YAML frontmatter (with local metadata)
   - Overview and features
   - Installation guide
   - Usage examples
   - API reference (if applicable)
   - Troubleshooting
   - Resources
8. Asks user where to save:
   - `./.opencode/skills/project-name/SKILL.md`
   - `~/.config/opencode/skills/project-name/SKILL.md`
   - `~/.claude/skills/project-name/SKILL.md`
9. Writes the file
10. Done!

### Why It Works

- ✅ Pure markdown skill - no dependencies
- ✅ Uses your configured LLM
- ✅ Leverages built-in tools
- ✅ Mirror rotation built into prompts
- ✅ Works across OpenCode/Claude Code/Cursor

## Extending the Skill

To add features:

1. **New Mirror**: Add to references/github-api.md
2. **New Platform**: Create references/platform-api.md
3. **New LLM Prompts**: Add to SKILL.md instructions
4. **Helper Functions**: Add to scripts/utils.sh

## Debugging

If the skill doesn't work:

1. Check the skill is in correct directory
2. Verify OpenCode/Claude Code recognizes it
3. Check internet connection
4. Verify repository exists and is public
5. Check LLM provider is configured correctly

## Performance

### Remote Repository
Expected times:
- Small repo (< 500 files): 30-60s
- Medium repo (500-2k files): 1-2min
- Large repo (2k+ files): 2-5min

Factors affecting speed:
- Mirror response times
- Repository size
- README length
- LLM speed (model, provider)
- Network connectivity

### Local Repository (New)
Expected times:
- Small project (< 500 files): 10-30s (faster, no network)
- Medium project (500-2k files): 30-60s
- Large project (2k+ files): 1-3min

Factors affecting speed:
- File system I/O speed
- Project size
- Number of configuration files
- README length
- LLM speed (model, provider)

**Advantage**: Local analysis is typically **2-3x faster** than remote (no network latency, no mirror rotation)

## Security

- No user data sent to external services except open APIs
- No secrets stored in the skill
- Uses your trusted LLM provider
- HTTPS used for all API calls

## License

MIT - Feel free to modify and extend!
