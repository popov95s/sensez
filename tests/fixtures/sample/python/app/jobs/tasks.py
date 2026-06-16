def task(fn):
    return fn


@task
def nightly_reindex():
    return "scheduled"


def setup():
    return "registered"
