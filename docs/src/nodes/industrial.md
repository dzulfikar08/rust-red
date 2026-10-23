# Industrial Nodes

## Modbus

Rust-RED supports Modbus TCP and Serial/RTU protocols with read, write, flex getter/writer, and server nodes.

### Modbus Config (Connection)

Shared connection configuration for all Modbus nodes.

**Type**: `modbus-config`

**Properties**:
- `transport` - "tcp" or "serial"/"rtu"
- `host` - TCP host (TCP mode)
- `port` - TCP port (default: 502)
- `serial_port` - serial device path (RTU mode, e.g., `/dev/ttyUSB0`)
- `baud_rate` - baud rate (default: 9600)
- `data_bits` - data bits (5-8, default: 8)
- `stop_bits` - stop bits (1 or 2, default: 1)
- `parity` - "none", "even", "odd"
- `unit_id` - Modbus slave address (1-247)
- `auto_connect` - auto-connect on first request (default: true)
- `reconnect_timeout` - reconnect delay in ms
- `command_delay` - minimum delay between requests in ms
- `parallel_unit_ids` - allow concurrent requests (default: false)

### Modbus Read

Reads from Modbus devices on a schedule.

**Type**: `modbus-read`

**Properties**:
- `server` - Modbus config node ID
- `fc` - function code: "FC1" (coils), "FC2" (discrete inputs), "FC3" (holding registers), "FC4" (input registers)
- `unitid` - unit/slave address override
- `address` - starting register/coil address
- `quantity` - number of registers/coils to read
- `poll_rate` - poll interval in seconds

### Modbus Write

Writes to Modbus devices.

**Type**: `modbus-write`

**Properties**:
- `server` - Modbus config node ID
- `fc` - function code: "FC5" (single coil), "FC6" (single register), "FC15" (multiple coils), "FC16" (multiple registers)
- `unitid` - unit/slave address override
- `address` - starting register/coil address
- `quantity` - number of registers/coils

### Modbus Flex Getter / Writer

Dynamic Modbus operations where parameters come from the message.

**Types**: `modbus-flex-getter`, `modbus-flex-writer`

**Properties**:
- `server` - Modbus config node ID

**Message properties**:
- `msg.payload.fc` - function code
- `msg.payload.unitid` - slave address
- `msg.payload.address` - start address
- `msg.payload.quantity` - count
- `msg.payload.value` - value to write (writer only)

### Modbus Server

Runs a Modbus TCP slave server.

**Type**: `modbus-server`

**Properties**:
- `port` - listen port (default: 10502)
- `coils` - initial coil values
- `holding_registers` - initial register values

## OPC-UA

### OPC-UA Config

**Type**: `opcua-config`

**Properties**:
- `endpoint` - OPC-UA server URL
- `security_mode` - "none", "sign", "signAndEncrypt"
- `security_policy` - security policy

### OPC-UA Read / Write

**Types**: `opcua-read`, `opcua-write`

**Properties**:
- `server` - OPC-UA config node ID
- `nodeId` - OPC-UA node ID
- `datatype` - expected data type

## BACnet

### BACnet Config

**Type**: `bacnet-config`

**Properties**:
- `deviceId` - BACnet device instance number of the target
- `targetHost` - IP address of the device (default `127.0.0.1`)
- `targetPort` - UDP port of the device (default: 47808)
- `apduTimeoutMs` - time to wait for a reply, per attempt (default: 3000)
- `retries` - resends after a timeout (default: 3)

### BACnet Read

**Type**: `bacnet read`

Reads properties of one object over BACnet/IP (ReadProperty, unicast to `targetHost:targetPort`) each time a message arrives. Use an `inject` node with a repeat to poll. Segmented replies, such as `objectList`, are reassembled.

**Properties**:
- `configNode` - BACnet config node ID
- `objectType` - `analogInput`, `analogOutput`, `analogValue`, `binaryInput`, `binaryOutput`, `binaryValue`, `multiStateInput`, `multiStateOutput`, `multiStateValue`, `device` and the other BACnet object types by name, or the type number
- `objectInstance` - object instance number
- `property` - property name (`presentValue`, the default, `objectName`, `units`, `statusFlags`, ...; `present-value` and `present_value` work too) or number. A comma separated list reads several properties.
- `properties` - more property names to read in the same message

**Output**: `msg.payload` is the value of the property, or, when more than one property is requested, an object keyed by property name. Real, Double, Unsigned, Signed and Enumerated values are numbers (binary present values are 0 or 1), Boolean is a boolean, character strings are strings, bit strings such as `statusFlags` are arrays of booleans, and constructed values are arrays. `msg.bacnet` holds `objectType`, `objectInstance`, `property`, `properties`, `endpoint` and `deviceId`. A device Error, Reject or Abort, or no reply, raises a node error that a `catch` node receives; no message is sent on the output.

**COV**: with `subscribeCov` the node also sends SubscribeCOV (unconfirmed notifications) for the object and emits a message for every notification that carries one of the requested properties, starting with the one the device sends right after subscribing. `covLifetime` is the lifetime in seconds (default: the config node's `covLifetime`, else 300); the subscription is renewed at half the lifetime and re-established after an error. `covIncrement` drops numeric changes smaller than the increment, except the first. Notification messages have `msg.bacnet.cov = true`, every property of the notification in `msg.bacnet.values` and the seconds left in `msg.bacnet.timeRemaining`. Input messages still trigger a read. Confirmed COV notifications are acknowledged.

### BACnet Write

**Type**: `bacnet write`

Writes `msg.payload` to one property with WriteProperty each time a message arrives.

**Properties**:
- `configNode`, `objectType`, `objectInstance`, `property` - as for `bacnet read` (`presentValue` is the default property, `analogOutput` the default object type)
- `priority` - command priority 1 (highest) to 16 for a commandable property; empty or 0 sends none and the device uses its default (16). `msg.priority` overrides it.
- `valueType` - BACnet type of the value: `auto` (default), `real`, `double`, `unsigned`, `signed`, `enumerated`, `boolean` or `string`. With `auto`, the present value of analog objects is written as Real, of binary objects as Enumerated (`true`, `false`, `1`, `0`, `"active"`, `"inactive"`) and of multi-state objects as Unsigned; other properties follow the type of the payload (boolean, integer, number, string).
- `relinquish` - write NULL at the priority instead of the payload, which releases that priority slot. `msg.relinquish = true` does the same, and so does a `null` payload. A priority is required.

**Output**: after the device acknowledges, `msg.payload` is `true` and `msg.bacnet` holds `objectType`, `objectInstance`, `property`, `endpoint`, `deviceId`, `priority`, `relinquished` and the `value` written. A wrong value, an unknown object or property, a device Error, Reject or Abort, or no reply raises a node error for a `catch` node; no message is sent on the output.
