# Contributing to Solana CLI Tool | Solana CLI Tool 贡献指南

## Development Process | 开发流程

1. Fork the repository | 复刻仓库
2. Create a feature branch | 创建功能分支
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. Make your changes | 进行更改
4. Run tests locally | 本地运行测试
   ```bash
   cargo test
   cargo clippy
   cargo fmt --all -- --check
   ```
5. Submit a pull request | 提交拉取请求

## Code Style | 代码风格

- Follow Rust style guidelines | 遵循 Rust 风格指南
- Use clippy for linting | 使用 clippy 进行代码检查
  ```bash
  cargo clippy -- -D warnings
  ```
- Format code with rustfmt | 使用 rustfmt 格式化代码
  ```bash
  cargo fmt
  ```

## Testing | 测试

- Add tests for new features | 为新功能添加测试
- Run tests before submitting PR | 提交 PR 前运行测试
  ```bash
  cargo test
  ```
- Ensure clippy passes with no warnings | 确保 clippy 检查无警告
- Test on multiple networks (Devnet, Testnet) | 在多个网络上测试（Devnet、Testnet）

## Pull Request Process | 拉取请求流程

1. Update documentation | 更新文档
   - Update README.md if adding features | 添加功能时更新 README.md
   - Add inline documentation for new code | 为新代码添加内联文档

2. Testing Requirements | 测试要求
   - Add unit tests for new functionality | 为新功能添加单元测试
   - Verify existing tests pass | 验证现有测试通过

3. Code Quality | 代码质量
   - Run clippy and address warnings | 运行 clippy 并解决警告
   - Format code using rustfmt | 使用 rustfmt 格式化代码

4. Review Process | 审查流程
   - Request review from maintainers | 请求维护者审查
   - Address review comments promptly | 及时处理审查意见
   - Keep PR scope focused and manageable | 保持 PR 范围集中且可管理

## Commit Guidelines | 提交指南

- Use clear and descriptive commit messages | 使用清晰描述性的提交信息
- Reference issues and PRs in commits | 在提交中引用问题和 PR
- Keep commits focused and atomic | 保持提交集中且原子化

## Getting Help | 获取帮助

- Open an issue for questions | 有问题请开启 issue
- Join discussions in existing issues | 加入现有问题的讨论
- Follow the code of conduct | 遵守行为准则

Thank you for contributing! | 感谢您的贡献！
