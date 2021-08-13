# Vers

Manage installs of dev tools, similar to ASDF except in Rust.

[![](https://mermaid.ink/svg/eyJjb2RlIjoiZ3JhcGggVERcbiAgICBBW0NocmlzdG1hc10gLS0-fEdldCBtb25leXwgQihHbyBzaG9wcGluZylcbiAgICBCIC0tPiBDe0xldCBtZSB0aGlua31cbiAgICBDIC0tPnxPbmV8IERbTGFwdG9wXVxuICAgIEMgLS0-fFR3b3wgRVtpUGhvbmVdXG4gICAgQyAtLT58VGhyZWV8IEZbZmE6ZmEtY2FyIENhcl1cbiAgIiwibWVybWFpZCI6eyJ0aGVtZSI6ImRlZmF1bHQifSwidXBkYXRlRWRpdG9yIjpmYWxzZSwiYXV0b1N5bmMiOnRydWUsInVwZGF0ZURpYWdyYW0iOmZhbHNlfQ)](https://mermaid-js.github.io/mermaid-live-editor/edit##eyJjb2RlIjoiZ3JhcGggVERcbiAgICBBW0NocmlzdG1hc10gLS0-fEdldCBtb25leXwgQihHbyBzaG9wcGluZylcbiAgICBCIC0tPiBDe0xldCBtZSB0aGlua31cbiAgICBDIDwtLT58T25lfCBEW0xhcHRvcF1cbiAgICBDIC0tPnxUd298IEVbaVBob25lXVxuICAgIEMgLS0-fFRocmVlfCBGW2ZhOmZhLWNhciBDYXJdXG4gICIsIm1lcm1haWQiOiJ7XG4gIFwidGhlbWVcIjogXCJkZWZhdWx0XCJcbn0iLCJ1cGRhdGVFZGl0b3IiOmZhbHNlLCJhdXRvU3luYyI6dHJ1ZSwidXBkYXRlRGlhZ3JhbSI6ZmFsc2V9)

## Concepts

### Environment

An environment is a collection of [`Tools`](#tool) and [`Versions`](#version). Environments keep track of the tools and versions being managed for easy upgrades later using the `--update-all` command.

### ToolManager

Handles various functions required to install, delete or update a [`Tools`](#tool). 

### Tool

Installed by a tool manager in addition to the [`Version`](#version) 

### Version

A state of the tool version to install, typically `Latest` or `Specific`. Latest will keep the tool updated to the latest version available to the [`ToolManager`](#toolmanager)

## Tool Manager

A `ToolManager` trait

```rust
#[derive(Debug, Clone)]
#[async_trait]
pub trait ToolManager<T> 
  where T: Send + Sync + ToolManagerConfig
{
  fn async install_tool(&self, T) -> Result<ToolManagerError>;
  fn async update_tool(&self, T) -> Result<ToolManagerError>;
  fn async delete_tool(&self, T) -> Result<ToolManagerError>;
  fn async switch_tool_version(&self, T) -> Result<ToolManagerError>;
}
```

### Install New Tool Flow

```mermaid
sequenceDiagram
    User->>Cli: versm tool install -r 'ogham/exa'
    Cli->>Tool: GitHubReleaseManager.Install("ogham/exa", version.Latest)
    Tool->>ToolManager: GitHubReleaseManager.Install(version, tool)
    ToolManager->>+ExternalService: GitHubReleaseManager.get_latest_release("ogham/exa")
    ExternalService->>+ToolManager: GitHubReleaseManager.parse_release(resp.json())
    alt already-latest
      ToolManager->>User: Already at latest version of ogham/exa (v)
    else
      ToolManager->>+ExternalService: GitHubReleaseManager.get_release_assets(release, "v.10.1")
      ExternalService->>+ToolManager: GitHubReleaseManager.parse_release_assets(resp.json())
      ToolManager->>Tool: GitHubReleaseManager.download_assets(release, "v.10.1")
      alt successful-download
        Tool->>Cli: Report success. Version: v0.10.1, Manager:GitHubReleaseManager, Name:ogham/exa
        Cli->>User: GithubReleaseManager: Successfully installed oghman/exa version v0.10.1
      else
        Tool->>Cli: Report failure. Error:No assets found for this platform, Manager:GitHubReleaseManager, Name:ogham/exa
        Cli->>User: GithubReleaseManager: Successfully installed oghman/exa version v0.10.1
      end
    end
```

### Update Tool Flow

```mermaid
sequenceDiagram
    User->>Cli: versm tool update -r 'ogham/exa'
    Cli->>Tool: UpdateTool(GitHubReleaseManager, "ogham/exa")
    Tool->>ToolManager: GitHubReleaseManager.Update(version, tool)
    ToolManager->>ExternalService: GitHubReleaseManager.get_releases(tool.name)
    ExternalService->>ToolManager: JSON response
    ToolManager->>Tool: GitHubReleaseManager.parse_release(resp.json())
    Tool->>Cli: Current: v0.9.0, Latest: v0.10.1
    Cli->>Tool: GitHubReleaseManager.Install("ogham/exa", version.Specific("v0.10.1"))
    Tool->>ToolManager: GitHubReleaseManager.Install(version, tool)
    ToolManager->>ExternalService: GitHubReleaseManager.get_release_assets(release, "v.10.1")
    ExternalService->>ToolManager: GitHubReleaseManager.parse_assets_json(resp.json())
    ToolManager->>Tool: GitHubReleaseManager.download_assets(release, assets_data)
    Tool->>Cli: Report success. NewVersion: v0.10.1, OldVersion:v0.9.0, Manager:GitHubReleaseManager, Name:ogham/exa
    Cli->>User: GithubReleaseManager: Successfully updated oghman/exa to version v0.10.1 (replaced: v0.9.0)
```