"""Modbus TCP simulator for the smoke test: a PLC with holding registers 0..9 = 100, 200, ... 1000.

    pip install "pymodbus==3.6.9"
    python scripts/smoke/modbus_sim.py 5020
"""
import asyncio
import sys

from pymodbus.datastore import ModbusSequentialDataBlock, ModbusServerContext, ModbusSlaveContext
from pymodbus.server import StartAsyncTcpServer


async def main(port: int) -> None:
    registers = [100 * (i + 1) for i in range(10)] + [0] * 90
    # pymodbus keeps a one register offset: the block starts at 1 to serve addresses from 0
    store = ModbusSlaveContext(
        di=ModbusSequentialDataBlock(1, [False] * 100),
        co=ModbusSequentialDataBlock(1, [False] * 100),
        hr=ModbusSequentialDataBlock(1, registers),
        ir=ModbusSequentialDataBlock(1, registers),
    )
    # single=True answers every unit id with this store
    context = ModbusServerContext(slaves=store, single=True)
    await StartAsyncTcpServer(context=context, address=("127.0.0.1", port))


if __name__ == "__main__":
    asyncio.run(main(int(sys.argv[1]) if len(sys.argv) > 1 else 5020))
