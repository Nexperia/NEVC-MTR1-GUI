# Safety Precautions — NEVC-MTR1 GUI

> **⚠ PREVIEW SOFTWARE**
>
> This application is currently in **preview / early-access** status. It may contain
> bugs that could result in unexpected motor behaviour, including unintended starts,
> speed changes, or direction reversals. **Never leave the motor unattended while
> the software is running.** Always be prepared to cut power to the system
> immediately if anything behaves unexpectedly.

---

## 1 · Motor Handling

Ensure that the motor is **securely clamped down** and cannot move unexpectedly
during operation. This is crucial to prevent accidents or damage to the system.
Do not hold the motor by hand while it is powered.

## 2 · Operating Ranges

Verify that the motor's intended operating ranges (voltage, current, speed, etc.)
are supported by your specific setup. Operating the motor outside its specified
limits can lead to malfunction, overheating, or permanent damage to the motor or
driver board.

## 3 · Power Isolation

Implement appropriate measures to **quickly and safely isolate power** to the
system if needed. This may include:

- An emergency-stop switch wired in series with the supply.
- An easily accessible mains power switch or bench PSU enable button.
- Clear labelling of power connectors to avoid accidental re-connection.

Always know how to cut power before powering the system.

## 4 · General Electrical Safety

Always take reasonable precautions when handling electronic components:

- Wear appropriate personal protective equipment (PPE) where required.
- Keep the workspace tidy and free of loose conductors that could cause short circuits.
- Be cautious of exposed high-voltage rails, capacitor charge, and inrush currents.
- Disconnect power before making wiring changes.
- Do not exceed the voltage or current ratings of any component in the system.

---

*Nexperia accepts no liability for damage or injury arising from unsafe use of
this software or the associated hardware.*
