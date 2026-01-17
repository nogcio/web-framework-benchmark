from aiohttp import web

import asyncio
import multiprocessing
import os
import signal
import sys


HELLO_WORLD = "Hello, World!"


async def health(_: web.Request) -> web.Response:
    return web.Response(text="OK", content_type="text/plain")


async def plaintext(_: web.Request) -> web.Response:
    return web.Response(text=HELLO_WORLD, content_type="text/plain")


def create_app() -> web.Application:
    app = web.Application()
    app.router.add_get("/health", health)
    app.router.add_get("/", plaintext)
    app.router.add_get("/plaintext", plaintext)
    return app


def _detect_cpu_count() -> int:
    # Use scheduler affinity to determine real CPU count available in a container.
    try:
        return len(os.sched_getaffinity(0))
    except AttributeError:
        return multiprocessing.cpu_count()


async def _serve(port: int, *, reuse_port: bool) -> None:
    app = create_app()
    runner = web.AppRunner(app, access_log=None)
    await runner.setup()

    site = web.TCPSite(
        runner,
        host="0.0.0.0",
        port=port,
        backlog=65535,
        reuse_port=reuse_port,
    )
    await site.start()
    await asyncio.Event().wait()


def _run_worker(port: int, *, reuse_port: bool) -> None:
    asyncio.run(_serve(port, reuse_port=reuse_port))


if __name__ == "__main__":
    port = int(os.getenv("PORT", "8080"))

    workers_env = os.getenv("WORKERS") or os.getenv("WEB_CONCURRENCY")
    if workers_env is not None:
        try:
            workers = int(workers_env)
        except ValueError:
            workers = 1
    else:
        workers = _detect_cpu_count()

    if workers <= 1:
        _run_worker(port, reuse_port=False)
        raise SystemExit(0)

    procs: list[multiprocessing.Process] = []

    def _shutdown(_: int, __) -> None:
        for p in procs:
            if p.is_alive():
                p.terminate()
        for p in procs:
            p.join(timeout=5)
            if p.is_alive():
                p.kill()
        sys.exit(0)

    signal.signal(signal.SIGTERM, _shutdown)
    signal.signal(signal.SIGINT, _shutdown)

    for _ in range(workers):
        p = multiprocessing.Process(target=_run_worker, args=(port,), kwargs={"reuse_port": True})
        p.start()
        procs.append(p)

    for p in procs:
        p.join()
