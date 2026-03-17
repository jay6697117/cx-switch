#!/bin/bash

# GitHub API Mirrors (in priority order)
GITHUB_API_MIRRORS=(
    "https://api.github.com"
    "https://gh.api.888888888.xyz"
    "https://gh-proxy.com/api/github"
    "https://api.fastgit.org"
    "https://api.kgithub.com"
    "https://githubapi.muicss.com"
    "https://github.91chi.fun"
    "https://mirror.ghproxy.com"
)

# Raw file mirrors
GITHUB_RAW_MIRRORS=(
    "https://raw.githubusercontent.com"
    "https://raw.fastgit.org"
    "https://raw.kgithub.com"
)

# Function: Try multiple mirrors for GitHub API
github_api_fetch() {
    local endpoint=$1
    local token=$2
    
    for mirror in "${GITHUB_API_MIRRORS[@]}"; do
        local url="${mirror}${endpoint}"
        
        if [ -n "$token" ]; then
            response=$(curl -s -H "Authorization: token ${token}" -H "Accept: application/vnd.github.v3+json" "$url" 2>&1)
        else
            response=$(curl -s -H "Accept: application/vnd.github.v3+json" "$url" 2>&1)
        fi
        
        # Check for success
        if [[ ! $response =~ "error" ]] && [[ ! $response =~ "403" ]] && [[ ! $response =~ "429" ]]; then
            echo "$response"
            return 0
        fi
    done
    
    return 1
}

# Function: Try multiple mirrors for raw files
github_raw_fetch() {
    local path=$1
    
    for mirror in "${GITHUB_RAW_MIRRORS[@]}"; do
        local url="${mirror}${path}"
        response=$(curl -s -L "$url" 2>&1)
        
        if echo "$response" | grep -q "<!DOCTYPE html>"; then
            continue
        fi
        
        if [ ${#response} -gt 100 ]; then
            echo "$response"
            return 0
        fi
    done
    
    return 1
}

# Function: Extract repo info from URL
parse_repo_info() {
    local url=$1

    # GitHub
    if [[ $url =~ github\.com/([^/]+)/([^/\?#]+) ]]; then
        echo "github ${BASH_REMATCH[1]} ${BASH_REMATCH[2]}"
        return 0
    fi

    # GitLab
    if [[ $url =~ gitlab\.com/([^/]+)/([^/\?#]+) ]]; then
        echo "gitlab ${BASH_REMATCH[1]} ${BASH_REMATCH[2]}"
        return 0
    fi

    # Gitee
    if [[ $url =~ gitee\.com/([^/]+)/([^/\?#]+) ]]; then
        echo "gitee ${BASH_REMATCH[1]} ${BASH_REMATCH[2]}"
        return 0
    fi

    return 1
}

# Function: Check if input is a local path
is_local_path() {
    local input=$1

    # Check if starts with ./ (relative path)
    if [[ $input =~ ^\./ ]]; then
        return 0
    fi

    # Check if starts with / (absolute path)
    if [[ $input =~ ^/ ]]; then
        return 0
    fi

    # Check if starts with ~ (home directory)
    if [[ $input =~ ^~ ]]; then
        return 0
    fi

    # Check if directory exists (relative path)
    if [ -d "$input" ]; then
        return 0
    fi

    # Not a local path
    return 1
}

# Function: Parse local repository information
parse_local_info() {
    local path=$1

    # Validate directory exists
    if [ ! -d "$path" ]; then
        echo "ERROR: Directory not found: $path"
        return 1
    fi

    # Get absolute path
    local abs_path
    abs_path=$(cd "$path" && pwd 2>/dev/null)
    if [ -z "$abs_path" ]; then
        echo "ERROR: Unable to get absolute path for: $path"
        return 1
    fi

    # Get repository name (directory name)
    local repo_name
    repo_name=$(basename "$abs_path")

    # Get git remote URL if available
    local git_remote=""
    if [ -d "$abs_path/.git" ]; then
        git_remote=$(cd "$abs_path" && git remote get-url origin 2>/dev/null || echo "")
    fi

    # Format output: type absolute_path repo_name git_remote
    echo "local ${abs_path} ${repo_name} ${git_remote}"
    return 0
}

# Export functions
export -f github_api_fetch
export -f github_raw_fetch
export -f parse_repo_info
export -f is_local_path
export -f parse_local_info
