# Gestión de cuentas de cobro

Este contexto organiza los documentos con los que una persona solicita a un cliente el pago de bienes o trabajos, y registra los abonos aplicados a cada documento.

## Language

**Cuenta de cobro**:
Documento emitido a un cliente para solicitar el pago de uno o más conceptos de cobro. Debe tener un total positivo, calculado como la suma de los subtotales de sus conceptos.
_Avoid_: Factura, recibo

**Concepto de cobro**:
Bien o trabajo incluido en una cuenta de cobro, expresado mediante una descripción, una cantidad positiva y un precio positivo. Pertenece a una única cuenta de cobro y su subtotal se redondea a dos decimales con la regla half-up.
_Avoid_: Ítem, línea

**Número de cuenta de cobro**:
Entero positivo y único que identifica una cuenta de cobro. Se sugiere de forma consecutiva, pero el usuario puede modificarlo mientras conserve su unicidad.
_Avoid_: ID de cuenta

**Fecha de emisión**:
Fecha calendario asignada a una cuenta de cobro. Puede corresponder al día actual, a una fecha pasada o a una fecha futura.
_Avoid_: Fecha de creación

**Cliente**:
Persona u organización a la que se emite una cuenta de cobro. Debe existir al momento de crear o editar la cuenta de cobro.
_Avoid_: Comprador, usuario

**Cuenta bancaria**:
Destino de pago indicado en una cuenta de cobro. Debe existir al momento de crear o editar la cuenta de cobro.
_Avoid_: Información bancaria

**Abono**:
Pago aplicado a una cuenta de cobro que reduce su saldo pendiente.
_Avoid_: Cuota

**Estado de pago**:
Clasificación de una cuenta de cobro como pendiente, parcialmente pagada o pagada. Se deriva exclusivamente de su total y de sus abonos; nunca se establece manualmente.
_Avoid_: Estado de la cuenta
