# aionix

接收用户的文本问题，然后利用 AI 模型（通过 rig 框架集成）生成答案，并将问答历史记录存储在 PostgreSQL 数据库中。

技术栈:

Web 框架: Actix Web - 一个高性能、功能丰富的 Rust Web 框架。

ORM: SeaORM - 一个异步、动态的 Rust ORM，与 Actix Web 的异步特性完美契合。

数据库: PostgreSQL - 一款强大、可靠的开源对象-关系数据库。

AI 框架: rig - 一个用于构建和部署 AI/ML 模型的 Rust 框架（请注意：rig 是一个相对较新且仍在发展中的框架，这里的示例将基于其核心概念进行构建）。