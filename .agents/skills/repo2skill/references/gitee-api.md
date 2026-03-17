# Gitee API v5 Quick Reference

## Base URL
https://gitee.com/api/v5

## Key Endpoints

Get Repository:
GET /repos/{owner}/{repo}
curl https://gitee.com/api/v5/repos/mindspore/docs

Get README:
GET /repos/{owner}/{repo}/readme?ref=master
curl "https://gitee.com/api/v5/repos/mindspore/docs/readme"

Get File Tree:
GET /repos/{owner}/{repo}/git/trees/{branch}?recursive=1
curl "https://gitee.com/api/v5/repos/mindspore/docs/git/trees/master?recursive=1"

## Authentication
-H "Authorization: token YOUR_TOKEN"

## curl Examples
# Get repo info
curl -s https://gitee.com/api/v5/repos/mindspore/docs

# Get README
curl -s "https://gitee.com/api/v5/repos/mindspore/docs/readme"

# Get file tree
curl -s "https://gitee.com/api/v5/repos/mindspore/docs/git/trees/master?recursive=1"
