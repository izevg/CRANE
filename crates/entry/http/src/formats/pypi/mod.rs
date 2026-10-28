//! Маршруты `PyPI` под `/pypi/{repo}`: hosted-репозитории (proxy pypi.org — отдельный шаг).

mod hosted;

pub(super) use hosted::hosted_router;
