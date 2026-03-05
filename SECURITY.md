# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | ✅ Active development |
| < 0.2   | ❌ Not supported     |

## Reporting a Vulnerability

If you discover a security vulnerability, please:

1. **Do not** open a public issue
2. Email: ahmed@abuiliazeed.com
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

I will respond within 48 hours and work on a fix.

## Security Considerations

### API Keys

mdsearch optionally uses OpenAI API for embeddings. API keys are:
- Read from environment variable `OPENAI_API_KEY` only
- Never logged or stored in the index
- Not included in any output files

### File Access

mdsearch reads markdown files from your filesystem:
- Only reads files you explicitly index
- Respects `.gitignore` by default
- Does not modify source files

### Index Data

The RocksDB index may contain:
- File paths
- File content chunks
- Embedding vectors (if generated)

Index data is stored locally in `.mdsearch/` directory.
