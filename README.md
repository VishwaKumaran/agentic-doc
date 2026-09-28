# Agentic-doc

`agentic-doc` est un outil en ligne de commande qui maintient la cohérence entre un projet logiciel et sa documentation.

## Installation

```bash
cargo install --path .
```

## Commandes

### 1. `setup`
Initialise la configuration du projet docs et le lie au projet code :
```bash
cd my-project-docs
agentic-doc setup ../my-project
```

### 2. `scan`
Analyse le projet et enregistre un snapshot de son état :
```bash
agentic-doc scan
```

### 3. `status`
Affiche les changements détectés depuis le dernier snapshot :
```bash
agentic-doc status
```

### 4. `docs`
Analyse l'impact des changements sur la documentation :
```bash
agentic-doc docs
```

## Exemple de `.agentic-doc/relations.json`

```json
{
  "version": 1,
  "relations": [
    {
      "source": "src/auth.py::AuthService",
      "target": "authentication.md",
      "origin": "explicit",
      "status": "validated",
      "confidence": "high"
    }
  ]
}
```
